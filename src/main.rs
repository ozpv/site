use axum::{
    Router,
    body::{self, Body},
    extract::{Request, State},
    handler::HandlerWithoutStateExt,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use site::{env::Env, state::AppState};
use std::{fs, path::PathBuf, time::Duration};
use tokio::net::TcpListener;
use tower::util::ServiceExt;
use tower_http::{
    compression::CompressionLayer, services::ServeDir, timeout::TimeoutLayer, trace::TraceLayer,
};

const ERR: (StatusCode, Html<&'static str>) =
    (StatusCode::NOT_FOUND, Html("<h1>404 Not Found</h1>"));

async fn not_found() -> impl IntoResponse {
    ERR
}

async fn inject(
    res: Response,
    component_name: &'static str,
    after: &'static str,
    state: AppState,
) -> impl IntoResponse {
    let (parts, body) = res.into_parts();

    let Ok(body) = body::to_bytes(body, 5_000_000).await else {
        return ERR.into_response();
    };

    let Ok(mut string) = String::from_utf8(body.to_vec()) else {
        return ERR.into_response();
    };

    let Some(index) = string.find(after) else {
        return ERR.into_response();
    };

    let path = state
        .get_dist_dir()
        .as_ref()
        .join("components")
        .join(component_name);

    let Ok(component) = fs::read_to_string(path) else {
        return ERR.into_response();
    };

    string.insert_str(index + after.len(), component.as_str());

    (parts, Body::from(string)).into_response()
}

async fn get_file(State(state): State<AppState>, mut req: Request) -> impl IntoResponse {
    if req.uri().path() == "/" {
        *req.uri_mut() = "/index.html".parse().map_err(|_| ERR)?;
    } else {
        let mut file_path = PathBuf::from(req.uri().path());

        if file_path.extension().is_none() {
            file_path.set_extension("html");
        }

        *req.uri_mut() = file_path
            .to_str()
            // in case this were to run on a windows machine
            .map(|s| s.replace('\\', "/"))
            .ok_or(ERR)?
            .parse()
            .map_err(|_| ERR)?;
    }

    ServeDir::new(state.get_dist_dir().as_ref())
        .precompressed_br()
        .precompressed_gzip()
        .fallback(not_found.into_service())
        .oneshot(req)
        .await
        .map_err(|_| ERR)
}

async fn get_try_inject_head_nav_footer(
    State(state): State<AppState>,
    req: Request,
) -> impl IntoResponse {
    let res = get_file(State(state.clone()), req).await.into_response();

    if !res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.contains("text/html"))
    {
        return res;
    }

    let res = inject(res, "head.html", "<html lang=\"en\">", state.clone())
        .await
        .into_response();
    let res = inject(res, "nav.html", "<body>", state.clone())
        .await
        .into_response();

    inject(res, "footer.html", "</main>", state.clone())
        .await
        .into_response()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let Env {
        site_addr,
        dist_dir,
    } = Env::get_or_default();

    let listener = TcpListener::bind(&site_addr).await?;

    tracing::info!("Listening on http://{site_addr}/");

    let router = Router::new()
        // every route is handled through fallback
        .fallback(get_try_inject_head_nav_footer)
        .layer(CompressionLayer::new().gzip(true))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(30),
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(AppState::new(dist_dir));

    axum::serve(listener, router).await?;

    Ok(())
}

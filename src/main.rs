use axum::{
    Router,
    extract::{Request, State},
    handler::HandlerWithoutStateExt,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, get_service},
};
use site::{env::Env, state::AppState};
use std::{path::PathBuf, time::Duration};
use tokio::net::TcpListener;
use tower::util::ServiceExt;
use tower_http::{
    compression::CompressionLayer,
    services::{ServeDir, ServeFile},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

const ERR: (StatusCode, Html<&'static str>) =
    (StatusCode::NOT_FOUND, Html("<h1>404 Not Found</h1>"));

async fn not_found() -> impl IntoResponse {
    ERR
}

async fn static_files(State(state): State<AppState>, req: Request) -> impl IntoResponse {
    ServeDir::new(state.get_dist_dir().as_ref())
        .precompressed_br()
        .precompressed_gzip()
        .fallback(not_found.into_service())
        .oneshot(req)
        .await
        .map_err(|_| ERR)
}

async fn html_files(State(state): State<AppState>, mut req: Request) -> impl IntoResponse {
    let mut file_path = PathBuf::from(req.uri().path());

    if file_path.extension().is_none() {
        file_path.set_extension("html");
    }

    *req.uri_mut() = file_path
        .to_str()
        // if this were to run on a windows machine
        .map(|s| s.replace("\\", "/"))
        .ok_or(ERR)?
        .parse()
        .map_err(|_| ERR)?;

    ServeDir::new(state.get_dist_dir().as_ref())
        .fallback(not_found.into_service())
        .oneshot(req)
        .await
        .map_err(|_| ERR)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let Env {
        site_addr,
        dist_dir,
    } = Env::get_or_default();

    let listener = TcpListener::bind(&site_addr).await?;

    tracing::info!("Listening on http://{site_addr}/");

    let router = Router::new()
        .route(
            "/",
            get_service(ServeFile::new(dist_dir.join("index.html"))),
        )
        .route("/assets/{*any}", get(static_files))
        .fallback(html_files)
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

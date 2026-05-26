use std::{env, path::PathBuf};

pub struct Env {
    pub site_addr: String,
    pub dist_dir: PathBuf,
}

impl Env {
    pub fn get_or_default() -> Self {
        let site_addr = env::var("SITE_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
        let dist_dir = env::var("DIST_DIR")
            .unwrap_or_else(|_| format!("{}/public", env!("CARGO_MANIFEST_DIR")))
            .into();

        Self {
            site_addr,
            dist_dir,
        }
    }
}

use axum::{Router, routing::get};
use crate::config::{Config};

pub mod r;

#[derive(Clone)]
pub struct AppState {
    configs: Config,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/r/{name}",
            get(r::get)
            .post(r::post)
        )
}
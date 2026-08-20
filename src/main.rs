use std::{collections::HashMap, println, sync::Arc};

use axum::Router;
use tokio::{fs::File, sync::RwLock};
use tower_http::cors::{Any, CorsLayer};

use crate::config::init_config;

pub mod aio;
pub mod config;
pub mod endpoint;
pub mod filepipe;
pub mod key_gen;

#[tokio::main]
async fn main() {
    let config = init_config(&String::from("config.lua")).unwrap();
    let state = endpoint::AppState {
        config,
        //open_read_files: Arc::new(RwLock::new(HashMap::new()))
        sessions: Arc::new(RwLock::new(HashMap::new())),
    };

    println!("{:#?}", state.config);

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", state.config.address, state.config.port))
            .await
            .unwrap();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
        ])
        .allow_headers(Any);

    let router: Router = Router::new()
        .merge(endpoint::routes())
        .with_state(state)
        .layer(cors);

    axum::serve(listener, router).await.unwrap();
}

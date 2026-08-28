use std::{collections::HashMap, println, sync::Arc};

use axum::Router;
use tokio::{fs::File, sync::RwLock};
use tower_http::cors::{Any, CorsLayer};

use config::init_config;

mod config;
mod endpoint;

#[tokio::main]
async fn main() {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let config = init_config(&String::from("config.lua")).unwrap();
    let state = crate::endpoint::AppState {
        config,
        //open_read_files: Arc::new(RwLock::new(HashMap::new()))
        sessions: Arc::new(RwLock::new(HashMap::new())),
        access_keys: Arc::new(RwLock::new(HashMap::new())),
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

    state.run_gc_keys_loop(std::time::Duration::from_secs(30));

    let router: Router = Router::new()
        .merge(endpoint::routes())
        .with_state(state)
        .layer(cors);

    axum::serve(listener, router).await.unwrap();
}

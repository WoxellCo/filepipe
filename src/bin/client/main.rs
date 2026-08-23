use crate::{caller::ClientState, config::init_config};

mod caller;
mod config;

#[tokio::main]
async fn main() {
    let config = init_config(&".fp/config.lua".to_string()).unwrap();

    let state = ClientState {
        client: reqwest::Client::new(),
        config: config.clone(),
        current_binding: config.bindings.get("media").cloned().unwrap(),
    };

    state.authenticate(None).await;

    println!("client!! 😭");
}

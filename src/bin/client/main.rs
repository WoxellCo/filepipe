use std::{println, process::exit};

use filepipe::filepipe::StreamType;

use crate::{caller::ClientState, config::init_config};

mod caller;
mod config;

struct Args {
    
}

#[tokio::main]
async fn main() {
    let config = init_config(&".fp/config.lua".to_string()).unwrap();

    let state = ClientState {
        client: reqwest::Client::new(),
        config: config.clone(),
        current_binding: config.bindings.get("media").cloned().unwrap(),
    };

    let key = match state.authenticate(None).await {
        Ok(key) => key,
        Err(error) => {
            println!("err: {:?}", error);
            exit(1);
        }
    };



    println!("client!! 😭");
}

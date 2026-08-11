use std::println;

use axum::Router;

use crate::config::init_config;

pub mod endpoint;
pub mod config;
pub mod filepipe;

#[tokio::main]
async fn main() {
    let config = init_config(&String::from("config.lua")).unwrap();

    println!("{:#?}", config);

    println!("Hello, world!");
}

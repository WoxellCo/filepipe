use crate::config::init_config;

mod config;

#[tokio::main]
async fn main() {
    let config = init_config(&".fp/config.lua".to_string());

    println!("client!! 😭");
}

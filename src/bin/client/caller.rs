use std::sync::Arc;

use axum::Json;
use reqwest::Client;

use crate::config::{Binding, Config};

pub struct ClientState {
    pub client: Client,
    pub config: Config,
    pub current_binding: Arc<Binding>,
}

impl ClientState {
    pub async fn authenticate(&self, username: Option<String>) -> Result<(), ()> {
        //let mut headers = HeaderMap::new();
        //headers.insert(AUTHORIZATION, HeaderValue::from_str(session_key).unwrap());

        let res = self
            .client
            .post(format!(
                "{}/a/{}",
                self.current_binding.remote_address,
                username.unwrap_or(self.current_binding.default_user.remote_username.clone())
            ))
            .send()
            .await;

        println!(
            "{:?}",
            serde_json::from_str::<serde_json::Value>(&res.unwrap().text().await.unwrap()).unwrap()
        );

        Ok(())
    }
}

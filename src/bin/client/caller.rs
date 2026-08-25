use std::{println, sync::Arc};

use axum::{Json, http::{HeaderMap, HeaderValue}};
use ed25519_dalek::Signer;
use filepipe::filepipe::StreamType;
use reqwest::{Client, Response, header::AUTHORIZATION};

use crate::{
    caller::SenderError::FailedToAuthenticate,
    config::{Binding, Config},
};

pub struct ClientState {
    pub client: Client,
    pub config: Config,
    pub current_binding: Arc<Binding>,
}

#[derive(Debug, Clone)]
pub enum SenderError {
    UserDoesNotExist { username: String },
    FailedToAuthenticate { message: Option<String> },
}

pub type AccessKey = [u8; 16];

/*#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HuPutReq {
    pub files;
}*/

impl ClientState {
    pub async fn authenticate(
        &self,
        username: Option<String>
    ) -> Result<AccessKey, SenderError> {

        let user = match username {
            Some(username) => match self.config.users.get(&username) {
                Some(user) => user.clone(),
                None => return Err(SenderError::UserDoesNotExist { username }),
            },
            None => self.current_binding.default_user.clone(),
        };

        let response = self
            .client
            .post(format!(
                "{}/a/{}",
                self.current_binding.remote_address, user.remote_username
            ))
            .send()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate { message: Some(String::from("a")) })?
            .text()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate { message: Some(String::from("b")) })?;

        let response = serde_json::from_str::<serde_json::Value>(&response)
            .map_err(|_| SenderError::FailedToAuthenticate { message: Some(String::from("c")) })?;

        let access_key: String = match response.get("accessKey") {
            Some(key) => match key.as_str() {
                Some(key) => key.to_string(),
                None => {
                    return Err(SenderError::FailedToAuthenticate { message: Some(String::from("1")) });
                }
            },
            None => {
                return Err(SenderError::FailedToAuthenticate { message: Some(String::from("d")) });
            }
        };

        let access_key_bytes: AccessKey = access_key
            .as_bytes()
            .try_into()
            .map_err(|_| SenderError::FailedToAuthenticate { message: Some(String::from("e")) })?;

        let signed = user.priv_key.sign(&access_key_bytes).to_vec();

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&access_key).unwrap());

        let response = self
            .client
            .put(format!("{}/a", self.current_binding.remote_address))
            .headers(headers)
            .body(signed)
            .send()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate { message: Some(String::from("f")) })?;
            
        if !response.status().is_success() {
            return Err(SenderError::FailedToAuthenticate { message: Some(String::from("2")) });
        };

        let access_key = response
            .text()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate { message: Some(String::from("g")) })?;

        /*let key = match stream_type {
            StreamType::UpStream => {
                let response = self
                    .client
                    .post(format!(
                        "{}/hu/{}",
                        self.current_binding.remote_address,
                        self.current_binding.remote_repository_name
                    ))
                    .header("authentication", access_key)
                    .json();
            }
            StreamType::DownStream => {}
        };*/

        Ok(access_key_bytes)
    }
}

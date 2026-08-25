use std::sync::Arc;

use axum::Json;
use ed25519_dalek::Signer;
use filepipe::filepipe::StreamType;
use reqwest::{Client, Response};

use crate::{
    caller::SenderError::FailedToAuthenticate,
    config::{Binding, Config},
};

pub struct ClientState {
    pub client: Client,
    pub config: Config,
    pub current_binding: Arc<Binding>,
}

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
        username: Option<String>,
        stream_type: StreamType,
    ) -> Result<AccessKey, SenderError> {
        //let mut headers = HeaderMap::new();
        //headers.insert(AUTHORIZATION, HeaderValue::from_str(session_key).unwrap());

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
            .map_err(|_| SenderError::FailedToAuthenticate { message: None })?
            .text()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate { message: None })?;

        let response = serde_json::from_str::<serde_json::Value>(&response)
            .map_err(|_| SenderError::FailedToAuthenticate { message: None })?;

        let access_key: String = match response.get("accessKey") {
            Some(key) => key.to_string(),
            None => {
                return Err(SenderError::FailedToAuthenticate { message: None });
            }
        };

        let access_key_bytes: AccessKey = access_key
            .as_bytes()
            .try_into()
            .map_err(|_| SenderError::FailedToAuthenticate { message: None })?;

        let signed = user.priv_key.sign(&access_key_bytes);

        let response: Response = self
            .client
            .put(format!("{}/a", self.current_binding.remote_address))
            .body(signed.to_bytes())
            .send()
            .await;

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

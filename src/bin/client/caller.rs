use std::{format, str, sync::Arc, todo};

use axum::http::{HeaderMap, HeaderValue};
use ed25519_dalek::Signer;
use filepipe::filepipe::StreamType;
use reqwest::{Client, header::AUTHORIZATION};
use serde_json::Value;

use crate::config::{Binding, Config};

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
pub type SessionKey = [u8; 64];

/*#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HuPutReq {
    pub files;
}*/

impl ClientState {
    pub async fn authenticate(&self, username: Option<String>) -> Result<AccessKey, SenderError> {
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
            .map_err(|_| SenderError::FailedToAuthenticate {
                message: Some(String::from("a")),
            })?
            .text()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate {
                message: Some(String::from("b")),
            })?;

        let response = serde_json::from_str::<serde_json::Value>(&response).map_err(|_| {
            SenderError::FailedToAuthenticate {
                message: Some(String::from("c")),
            }
        })?;

        let access_key: String = match response.get("accessKey") {
            Some(key) => match key.as_str() {
                Some(key) => key.to_string(),
                None => {
                    return Err(SenderError::FailedToAuthenticate {
                        message: Some(String::from("1")),
                    });
                }
            },
            None => {
                return Err(SenderError::FailedToAuthenticate {
                    message: Some(String::from("d")),
                });
            }
        };

        let access_key_bytes: AccessKey =
            access_key
                .as_bytes()
                .try_into()
                .map_err(|_| SenderError::FailedToAuthenticate {
                    message: Some(String::from("e")),
                })?;

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
            .map_err(|_| SenderError::FailedToAuthenticate {
                message: Some(String::from("f")),
            })?;

        if !response.status().is_success() {
            return Err(SenderError::FailedToAuthenticate {
                message: Some(String::from("2")),
            });
        };

        let _ = response
            .text()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate {
                message: Some(String::from("g")),
            })?;

        Ok(access_key_bytes)
    }

    pub async fn send_open_stream_request(&self, stream_type: StreamType, access_key: AccessKey) -> Result<SessionKey, ()> {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(str::from_utf8(&access_key).unwrap()).unwrap());


        match stream_type {
            StreamType::UpStream => {
                let response = self
                    .client
                    .post(format!("{}/hu/{}", self.current_binding.remote_address, self.current_binding.remote_repository_name))
                    .headers(headers)
                    .send()
                    .await
                    .map_err(|_| ())?;

                if !response.status().is_success() {
                    todo!("return proper error");
                    return Err(());
                }

                let response: Value = response
                    .json()
                    .await
                    .map_err(|_| ())?;

                let key = response.get("key");
            }
            StreamType::DownStream => {}
        }

        Err(())
    }
}

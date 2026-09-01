use std::{format, println, str, sync::Arc, todo};

use axum::http::{HeaderMap, HeaderValue, response};
use ed25519_dalek::Signer;
use filepipe::{
    aio::{IOError, get_file_list_in_dir_with_fpignore},
    filepipe::{StreamType, pack_repository_files_info},
};
use reqwest::{Client, header::AUTHORIZATION};
use serde::de::value;
use serde_json::{Value, json};

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
    FailedToInitializeStream { message: String },
    IOError { error: IOError },
}

pub type AccessKey = [u8; 16];
pub type SessionKey = String; //[u8; 64];

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
                message: Some(String::from("the client couldn't fetch the post request")),
            })?;

        if !response.status().is_success() {
            let response: Value =
                response
                    .json()
                    .await
                    .map_err(|_| SenderError::FailedToAuthenticate {
                        message: Some(String::from(
                            "an unknown error has occurred during the post request",
                        )),
                    })?;

            match response.get("error") {
                Some(value) => {
                    return Err(SenderError::FailedToAuthenticate {
                        message: Some(value.as_str().unwrap_or("an error has occurred during the post request, additionally, the client couldn't extract the error message").to_string()),
                    });
                }
                None => {
                    return Err(SenderError::FailedToAuthenticate {
                        message: Some(
                            "an error has occurred during the post request, additionally, the client couldn't extract the error message"
                            .to_string()
                        )
                    });
                }
            }
        }

        let response: Value =
            response
                .json()
                .await
                .map_err(|_| SenderError::FailedToAuthenticate {
                    message: Some(String::from("the server sent an invalid response")),
                })?;

        println!("{:?}", response);

        let access_key: String = match response.get("accessKey") {
            Some(key) => match key.as_str() {
                Some(key) => key.to_string(),
                None => {
                    return Err(SenderError::FailedToAuthenticate {
                        message: Some(String::from(
                            "failed to read the access key sent by the server",
                        )),
                    });
                }
            },
            None => {
                return Err(SenderError::FailedToAuthenticate {
                    message: Some(String::from(
                        "the expected key from the server was not provided",
                    )),
                });
            }
        };

        let access_key_bytes: AccessKey =
            access_key
                .as_bytes()
                .try_into()
                .map_err(|_| SenderError::FailedToAuthenticate {
                    message: Some(String::from("an error occurred on the client side during the byte conversion for the access key")),
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
                message: Some(String::from("the client couldn't fetch the put request")),
            })?;

        if !response.status().is_success() {
            return Err(SenderError::FailedToAuthenticate {
                message: Some(String::from("corrupted access key")),
            });
        };

        let _ = response
            .text()
            .await
            .map_err(|_| SenderError::FailedToAuthenticate {
                message: Some(String::from("corrupted access key")),
            })?;

        Ok(access_key_bytes)
    }

    pub async fn send_open_stream_request(
        &self,
        stream_type: StreamType,
        access_key: AccessKey,
    ) -> Result<SessionKey, SenderError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(str::from_utf8(&access_key).unwrap()).unwrap(),
        );

        let session_key;

        println!(
            "self.current_binding.local_path: {}",
            self.current_binding.local_path
        );
        let entries = get_file_list_in_dir_with_fpignore(&self.current_binding.local_path)
            .await
            .map_err(|error| SenderError::IOError { error })?;

        let entries = pack_repository_files_info(entries);

        println!("{:?}", entries);
        println!("abc {:?}", stream_type);

        match stream_type {
            StreamType::UpStream => {
                let response = self
                    .client
                    .post(format!(
                        "{}/hu/{}",
                        self.current_binding.remote_address,
                        self.current_binding.remote_repository_name
                    ))
                    .headers(headers)
                    .send()
                    .await
                    .map_err(|_| SenderError::FailedToInitializeStream {
                        message: "idk1".to_string(),
                    })?;

                if !response.status().is_success() {
                    return Err(SenderError::FailedToInitializeStream {
                        message: response.text().await.unwrap_or("unknown".to_string()),
                    });
                }

                let response: Value =
                    response
                        .json()
                        .await
                        .map_err(|_| SenderError::FailedToInitializeStream {
                            message: "idk2".to_string(),
                        })?;

                let key = match response.get("key") {
                    Some(key) => match key.as_str() {
                        Some(key) => key,
                        None => {
                            todo!("return proper error");
                        }
                    },
                    None => {
                        todo!("return proper error");
                    }
                };

                session_key = key.to_string();

                let mut headers = HeaderMap::new();
                headers.insert(AUTHORIZATION, HeaderValue::from_str(&session_key).unwrap());

                let response = self
                    .client
                    .put(format!(
                        "{}/hu/{}",
                        self.current_binding.remote_address,
                        self.current_binding.remote_repository_name
                    ))
                    .headers(headers)
                    .json(&json!({
                        "files": entries
                    }))
                    .send()
                    .await
                    .map_err(|_| SenderError::FailedToInitializeStream {
                        message: "idk1".to_string(),
                    })?;

                if !response.status().is_success() {
                    return Err(SenderError::FailedToInitializeStream {
                        message: response.text().await.unwrap(),
                    });
                }
            }
            StreamType::DownStream => {
                session_key = String::new();
            }
        }

        Ok(session_key)
    }
}

use std::{collections::HashMap, format, println, str, sync::Arc, todo};

use axum::http::{HeaderMap, HeaderValue, response};
use ed25519_dalek::Signer;
use filepipe::{
    aio::{
        IOError, compose_path_dir_and_name, extract_path_dir_and_name,
        get_file_list_in_dir_with_fpignore,
    },
    filepipe::{RepositoryFile, StreamType, pack_repository_files_info},
};
use reqwest::{Client, header::AUTHORIZATION};
use serde::de::value;
use serde_json::{Value, json};
use tokio::sync::RwLock;

use crate::{
    config::{Binding, Config},
    stream::{FileStream, FileStreamStatus},
};

pub struct ClientState {
    pub client: Client,
    pub config: Config,
    pub current_binding: Arc<Binding>,
    pub session_key: Option<SessionKey>,
}

#[derive(serde::Deserialize, Debug)]
pub struct FileActionCounters {
    pub moved: usize,
    pub copied: usize,
    pub network: usize,
    pub deleted: usize,
}

#[derive(serde::Deserialize, Debug)]
pub struct FileActions {
    pub counters: FileActionCounters,
    pub transfer: Vec<String>,
    pub temps: HashMap<String, String>,
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

#[derive(Debug)]
pub struct OpenStreamRequestInfo {
    pub session_key: SessionKey,
    pub actions: FileActions,
    pub stream_type: StreamType,
    pub filtered_entries_for_network: Vec<RepositoryFile>,
}

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
        &mut self,
        stream_type: StreamType,
        access_key: AccessKey,
    ) -> Result<OpenStreamRequestInfo, SenderError> {
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
        let entries: Vec<RepositoryFile> =
            get_file_list_in_dir_with_fpignore(&self.current_binding.local_path)
                .await
                .map_err(|error| SenderError::IOError { error })?
                .iter()
                .map(|entry| entry.1.clone())
                .collect();

        //println!("CLIENT ENTRIES({:?})", entries);
        let packed_entries = pack_repository_files_info(entries.clone());

        //println!("{:?}", entries);
        //println!("abc {:?}", stream_type);

        let actions;

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
                        "files": packed_entries
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

                actions = response.json::<FileActions>().await.map_err(|err| {
                    println!("{:?}", err);
                    SenderError::FailedToInitializeStream {
                        message: "idk2".to_string(),
                    }
                })?;
            }
            StreamType::DownStream => {
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
                        "files": packed_entries
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

                actions = response.json::<FileActions>().await.map_err(|err| {
                    println!("{:?}", err);
                    SenderError::FailedToInitializeStream {
                        message: "idk2".to_string(),
                    }
                })?;
            }
        }

        self.session_key = Some(session_key.clone());

        println!("{:?}", actions.transfer);

        // mk: the filtered entries, are the entries taken from the `Vec<RepositoryFile>` that originally have
        //     the og paths, and those that don't get transferred (not present in `actions.network`) get filtered out,
        //     this process also converts the paths to temporary paths, and outputs the new `Vec<RepositoryFile>`
        let filtered_entries_for_network: Vec<RepositoryFile> = entries
            .iter()
            .filter_map(|entry| {
                let entry_path = compose_path_dir_and_name(&entry.path_dir, &entry.name);
                let entry = match actions
                    .temps
                    .iter()
                    .find(|(_temp_path, og_path)| **og_path == *entry_path)
                {
                    // mk: it's a bit confusing but this block is part of the match above (assigning `entry` with `.find()`)
                    // mk: the tuple in `Some(x)` is the (key, value) from the `temps`, key is temp path, value is og path,
                    Some(path) => {
                        let (path_dir, name) = extract_path_dir_and_name(path.0);
                        RepositoryFile {
                            path_dir,
                            name,
                            ..entry.clone()
                        }
                    }
                    None => entry.clone(),
                };

                let path = compose_path_dir_and_name(&entry.path_dir, &entry.name);

                println!("entry_path: {entry_path}");
                if actions.transfer.contains(&path) {
                    Some(entry)
                } else {
                    None
                }
            })
            //.cloned()
            .collect();

        Ok(OpenStreamRequestInfo {
            session_key,
            actions,
            stream_type,
            filtered_entries_for_network,
        })
    }

    pub async fn send_cancel_stream_request(&self) -> Result<(), ()> {
        let Some(session_key) = &self.session_key else {
            return Err(());
        };

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(session_key).unwrap());

        let response = self
            .client
            .delete(&self.current_binding.remote_address)
            .headers(headers);

        Ok(())
    }
}

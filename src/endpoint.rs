use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::{
    config::{Config, User},
    filepipe::{Repository, RepositoryAccess},
    key_gen,
};
use axum::{
    Router,
    http::HeaderValue,
    routing::{get, post, put},
};
use tokio::fs::File;

pub mod a;
pub mod hd;
pub mod hu;
pub mod i;
pub mod ss;

#[derive(Clone, PartialEq, Debug)]
pub enum StreamType {
    DownStream,
    UpStream,
}

#[derive(Clone, Debug)]
pub struct Session {
    pub stream_type: StreamType,
    pub repository: Arc<Repository>,
}

// authentication process, before the session key the server gives to the client an access key challenge the client has to sign
#[derive(Clone, Debug)]
pub struct AccessKey {
    //token: String,
    user: Arc<User>,
    signed: bool,
    expire: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    //pub open_read_files: Arc<RwLock<HashMap<String, File>>>, // mk: yeah doing this later
    pub sessions: Arc<RwLock<HashMap<String, Session>>>,
    pub access_keys: Arc<RwLock<HashMap<String, AccessKey>>>,
}

impl AppState {
    pub async fn find_active_upstream(&self, name: &str) -> Option<(String, Session)> {
        let sessions = self.sessions.read().await;
        for (session_key, session) in sessions.iter() {
            if session.stream_type != StreamType::UpStream {
                continue;
            }

            if session.repository.name == name {
                return Some((session_key.to_owned(), session.clone()));
            }
        }

        None
    }

    pub async fn register_upstream_session(&self, repository_name: &str) -> Result<String, ()> {
        let repository = self.config.repositories.get(repository_name);

        let repository = match repository {
            Some(repository) => repository.clone(),
            None => {
                return Err(());
            }
        };

        let existing_upstream = self.find_active_upstream(repository_name).await;

        match existing_upstream {
            Some(_) => Err(()),
            None => {
                let mut key;
                {
                    let sessions = self.sessions.read().await;
                    loop {
                        key = key_gen::generate_random_string(64);
                        if !sessions.contains_key(&key) {
                            break;
                        }
                    }
                }

                let mut sessions = self.sessions.write().await;
                sessions.insert(
                    key.to_owned(),
                    Session {
                        stream_type: StreamType::UpStream,
                        repository,
                    },
                );

                Ok(key)
            }
        }
    }
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/a", put(a::put))
        .route("/a/{username}", post(a::post))
        .route("/i/{name}", get(i::get).post(i::post))
        .route(
            "/ss/{name}/{*path}",
            get(ss::get).post(i::post).put(i::post),
        )
        .route("/hu/{name}", post(hu::post).put(hu::put))
}

impl AppState {
    pub async fn get_session_by_key_in_header_value(
        &self,
        key: Option<&HeaderValue>,
    ) -> Option<Session> {
        let key = match key {
            Some(key) => String::from(key.to_str().ok()?.to_string()),
            None => {
                return None;
            }
        };

        let sessions = self.sessions.read().await;
        let session: Option<&Session> = sessions.get(&key);
        //println!("{key}");
        session.cloned()
    }

    pub fn authenticate(&self, key: Option<&HeaderValue>) -> Option<RepositoryAccess> {
        let key = match key {
            Some(key) => String::from(key.to_str().unwrap()),
            None => {
                return None;
            }
        };

        None
    }
}

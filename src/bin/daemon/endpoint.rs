use std::{collections::HashMap, sync::Arc, time};

use chrono::{DateTime, Utc};
use filepipe::filepipe::{RepositoryFile, StreamType};
use tokio::sync::RwLock;

use axum::{
    Router,
    http::HeaderValue,
    routing::{get, post, put},
};
use tokio::fs::File;
use {
    crate::config::{Config, User},
    filepipe::filepipe::{Repository, RepositoryAccess},
    filepipe::keys,
};

pub mod a;
pub mod hd;
pub mod hu;
pub mod i;
pub mod ss;

#[derive(Clone, Debug)]
pub struct Session {
    pub stream_type: StreamType,
    pub repository: Arc<Repository>,
    pub expire: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub file_list: Vec<RepositoryFile>,
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
                        key = keys::generate_random_string(64);
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
                        expire: Utc::now() + chrono::Duration::days(1),
                        last_activity: Utc::now(),
                        file_list: Vec::new(),
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
    ) -> (Option<String>, Option<Session>) {
        let key = match key {
            Some(key) => match key.to_str() {
                Ok(key) => key.to_string(),
                Err(_) => return (None, None),
            },
            None => {
                return (None, None);
            }
        };

        let sessions = self.sessions.read().await;
        let session: Option<&Session> = sessions.get(&key);
        //println!("{key}");
        (Some(key.clone()), session.cloned())
    }

    pub async fn authenticate(
        &self,
        key: Option<&HeaderValue>,
        repository_name: &str,
    ) -> Option<RepositoryAccess> {
        let mut access_sessions = self.access_keys.write().await;
        let key = match key {
            Some(key) => String::from(key.to_str().unwrap()),
            None => {
                return None;
            }
        };

        let access_key = match access_sessions.get(&key) {
            Some(access_key) => access_key.clone(),
            None => return None,
        };

        access_sessions.remove(&key);

        if access_key.is_expired() {
            return None;
        }

        if !access_key.signed {
            return None;
        }

        let user = access_key.user.clone();

        let repository = match self.config.repositories.get(repository_name) {
            Some(repository) => repository,
            None => return None,
        };

        let access = match repository.access_list.get(&user.name) {
            Some(access) => access,
            None => return None,
        };

        Some(access.clone())
    }

    pub fn run_gc_keys_loop(&self, every: time::Duration) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(every);
            loop {
                interval.tick().await;
                state.gc_keys().await;
            }
        });
    }

    pub async fn gc_keys(&self) {
        let expired_access_keys: Vec<String>;
        {
            let access_keys = self.access_keys.read().await;

            expired_access_keys = access_keys
                .iter()
                .filter(|(_, access_key)| access_key.is_expired())
                .map(|(k, _)| k.clone())
                .collect();
        }

        let expired_sessions: Vec<String>;
        {
            let sessions = self.sessions.read().await;

            expired_sessions = sessions
                .iter()
                .filter(|(_, session)| session.is_expired())
                .map(|(k, _)| k.clone())
                .collect();
        }

        {
            let mut access_keys = self.access_keys.write().await;
            for expired in expired_access_keys.iter() {
                access_keys.remove(expired);
            }
        }

        {
            let mut sessions = self.sessions.write().await;
            for expired in expired_sessions.iter() {
                sessions.remove(expired);
            }
        }
    }

    pub async fn with_session_mut<F, R>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(&mut Session) -> R,
    {
        let mut sessions = self.sessions.write().await;
        sessions.get_mut(key).map(f)
    }
}

pub trait Expirable {
    fn is_expired(&self) -> bool;
}

impl Session {
    pub fn update_last_activity(&mut self) {
        self.last_activity = Utc::now();
    }
}

impl Expirable for Session {
    fn is_expired(&self) -> bool {
        let now = Utc::now();
        now > self.expire || now > self.last_activity + chrono::Duration::minutes(30)
    }
}

impl Expirable for AccessKey {
    fn is_expired(&self) -> bool {
        let now = Utc::now();
        now > self.expire + chrono::Duration::minutes(5)
    }
}

use std::{collections::HashMap, sync::{Arc, RwLock}};

use axum::{Router, http::HeaderValue, routing::get};
use tokio::fs::File;
use crate::{config::Config, filepipe::Repository};

pub mod i;
pub mod hu;
pub mod hd;
pub mod ss;

#[derive(Clone, PartialEq)]
pub enum StreamType {
    DownStream,
    UpStram,
}

#[derive(Clone)]
pub struct Session {
    pub stream_type: StreamType,
    pub repository: Arc<Repository>,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    //pub open_read_files: Arc<RwLock<HashMap<String, File>>>, // mk: yeah doing this later
    pub sessions: HashMap<String, Session>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/i/{name}",
            get(i::get)
            .post(i::post)
        )
        .route("/ss/{name}/{*path}",
            get(ss::get)
            .post(i::post)
            .put(i::post)
        )
}

impl AppState {
    pub fn get_session_by_key_in_header_value(&self, key: Option<&HeaderValue>) -> Option<&Session> {
        let key = match key {
            Some(key) => String::from(key.to_str().unwrap()),
            None => {
                return None;
            }
        };

        let session: Option<&Session> = self.sessions.get(&key);
        //println!("{key}");
        session
    }
}
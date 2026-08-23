use std::collections::HashMap;
use tokio::time::Instant;

#[derive(Clone, Debug)]
pub struct RepositoryAccessAttribute {
    pub read: bool,
    pub write: bool,
}

#[derive(Clone, Debug)]
pub struct RepositoryAccess {
    pub info: RepositoryAccessAttribute,
    pub content: RepositoryAccessAttribute,
}

#[derive(Clone, Debug)]
pub struct Repository {
    pub name: String,
    pub path: String,
    pub access_list: HashMap<String, RepositoryAccess>,
}

pub struct RepositoryMeta {
    pub updated_on: Instant,
    pub previously_updated_on: Instant,
}

#[derive(Default)]
pub struct RepositoryFile {
    pub path_dir: String,
    pub name: String,
    pub size: usize,
    pub hash: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum StreamType {
    DownStream,
    UpStream,
}

pub fn extract_path_dir_and_name(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((dir, name)) => (dir.to_owned(), name.to_owned()),
        None => (String::new(), path.to_owned()),
    }
}

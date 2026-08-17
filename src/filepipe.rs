use std::collections::HashMap;
use tokio::time::Instant;

#[derive(Clone, Debug)]
pub struct RepositoryAccessAttribute {
    pub read: bool,
    pub write: bool
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
    pub access_list: HashMap<String, RepositoryAccess>
}

pub struct RepositoryMeta {
    pub updated_on: Instant,
    pub previously_updated_on: Instant
}

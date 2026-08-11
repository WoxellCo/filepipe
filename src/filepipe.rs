use std::collections::HashMap;

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
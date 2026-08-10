use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct RepositoryPermissionsAttribute {
    pub read: bool,
    pub write: bool
}

#[derive(Clone, Debug)]
pub struct RepositoryPermissions {
    pub info: RepositoryPermissionsAttribute,
    pub content: RepositoryPermissionsAttribute,
}

#[derive(Clone, Debug)]
pub struct Repository {
    pub name: String,
    pub access: HashMap<String, RepositoryPermissions>
}
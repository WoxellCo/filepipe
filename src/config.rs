use crate::filepipe::{Repository, RepositoryAccess, RepositoryAccessAttribute};
use mlua::{Error::*, Lua, StdLib, Table};
use std::{collections::HashMap, fs::read_to_string, sync::Arc, vec};

#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    pub pub_key_path: String,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub users: HashMap<String, User>,
    pub repository_list_path: String,
    pub repositories: HashMap<String, Arc<Repository>>,
    pub address: String,
    pub port: u16,
}

#[derive(Clone, Debug)]
pub enum ConfigError {
    FailedToLoadLuaLibs,
    FailedToLoadUsers,
    InvalidUserPubKeyPath { name: String },
    NoRepositoryList,
    FailedToLoadFileForRepositoryList { path: String },
    LuaErrorForRepositoryList { message: String },
    LuaUnknownErrorForRepositoryList,
    FailedToLoadRepositoryList,
    LuaErrorForConfig { message: String },
    LuaUnknownErrorForConfig,
    FailedToLoadFileForConfig { path: String },
    InvalidRepositoryPath { repository_name: String },
    NoAddress,
    NoPort,
}

fn load_repository_access(_username: &String, access_values: &Table) -> RepositoryAccess {
    let get_attributes = |section: &str| -> RepositoryAccessAttribute {
        match access_values.get::<Table>(section) {
            Ok(t) => RepositoryAccessAttribute {
                read: t.get("read").unwrap_or(false),
                write: t.get("write").unwrap_or(false),
            },
            Err(_) => RepositoryAccessAttribute {
                read: false,
                write: false,
            },
        }
    };

    RepositoryAccess {
        info: get_attributes("info"),
        content: get_attributes("content"),
    }
}

fn load_repository(key: &String, repository_table: &Table) -> Result<Repository, ConfigError> {
    let repository_path =
        repository_table
            .get::<String>("path")
            .map_err(|_| ConfigError::InvalidRepositoryPath {
                repository_name: key.clone(),
            })?;

    let access_table = repository_table.get::<Table>("access_list");
    let mut access_list = HashMap::new();

    // mk: i think there's a better way to do this
    let _ = access_table.and_then(|table| {
        let _ = table.for_each(|k: String, v| {
            access_list.insert(k.clone(), load_repository_access(&k, &v));
            Ok(())
        });
        Ok(())
    });

    let repository = Repository {
        name: key.clone(),
        path: repository_path,
        access_list,
    };

    Ok(repository)
}

fn load_repositories(
    path: &String,
    lua: &Lua,
    errors: &mut Vec<ConfigError>,
) -> Result<HashMap<String, Arc<Repository>>, ConfigError> {
    let content =
        read_to_string(path).map_err(|_| ConfigError::FailedToLoadFileForRepositoryList {
            path: path.to_owned(),
        })?;

    let chunk = lua.load(content);
    chunk
        .exec() //.unwrap();
        .map_err(|err| match err {
            SyntaxError {
                message,
                incomplete_input: _,
            } => ConfigError::LuaErrorForRepositoryList { message },
            RuntimeError(message) => ConfigError::LuaErrorForRepositoryList { message },
            _ => ConfigError::LuaUnknownErrorForRepositoryList,
        })?;

    let glob = lua.globals();
    let repositories_table = glob
        .get::<Table>("repositories")
        .map_err(|_| ConfigError::FailedToLoadRepositoryList)?;

    let mut repositories: HashMap<String, Arc<Repository>> = HashMap::new();

    // mk: uhhhh...
    let _ = repositories_table.for_each(|k: String, v: Table| {
        match load_repository(&k, &v) {
            Ok(repository) => {
                repositories.insert(k, Arc::new(repository));
            }
            Err(err) => {
                errors.push(err);
            }
        }
        Ok(())
    });

    Ok(repositories)
}

pub fn init_config(path: &String) -> Result<Config, Vec<ConfigError>> {
    let mut errors: Vec<ConfigError> = Vec::new();

    let lua = Lua::new();

    let content = read_to_string(path).map_err(|_| {
        vec![ConfigError::FailedToLoadFileForConfig {
            path: path.to_owned(),
        }]
    })?;

    let chunk = lua.load(content);
    chunk.exec().map_err(|err| match err {
        SyntaxError {
            message,
            incomplete_input: _,
        } => vec![ConfigError::LuaErrorForConfig { message }],
        _ => vec![ConfigError::LuaUnknownErrorForConfig],
    })?;

    lua.load_std_libs(StdLib::ALL_SAFE)
        .map_err(|_| vec![ConfigError::FailedToLoadLuaLibs])?;

    let glob = lua.globals();
    let users = glob
        .get::<Table>("users")
        .map_err(|_| vec![ConfigError::FailedToLoadUsers])?;

    let address = glob
        .get::<String>("server_address")
        .map_err(|_| vec![ConfigError::NoAddress])?;

    let port = glob
        .get::<u16>("server_port")
        .map_err(|_| vec![ConfigError::NoPort])?;

    let mut users_map: HashMap<String, User> = HashMap::new();

    let _ = users.for_each(|k: String, v: Table| {
        let mut user_pub_key_path: Option<String> = None;
        match v.get::<String>("pub_key_path") {
            Ok(user_key) => user_pub_key_path = Some(user_key),
            Err(_) => {
                // mk: no need to clone? because in this case it returns before `k` is used again? but it errors so...
                errors.push(ConfigError::InvalidUserPubKeyPath { name: k.clone() });
            }
        };
        let user_pub_key_path = match user_pub_key_path {
            Some(pub_key_path) => pub_key_path,
            None => {
                return Ok(());
            }
        };
        // mk: handle other attribs here or idk, but do it before inserting into the hashmap
        users_map.insert(
            k.clone(),
            User {
                name: k,
                pub_key_path: user_pub_key_path,
            },
        );
        Ok(())
    });

    let repository_list_path = match glob.get::<String>("repository_list") {
        Ok(path) => path,
        Err(_) => {
            errors.push(ConfigError::NoRepositoryList);
            return Err(errors);
        }
    };

    let repositories = match load_repositories(&repository_list_path, &lua, &mut errors) {
        Ok(list) => list,
        Err(err) => {
            errors.push(err);
            return Err(errors);
        }
    };

    if errors.len() > 0 {
        return Err(errors);
    }

    let config = Config {
        users: users_map,
        repository_list_path,
        repositories,
        address,
        port,
    };

    Ok(config)
}

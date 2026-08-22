use mlua::{Error::*, Lua, StdLib, Table};
use std::{collections::HashMap, fs::read_to_string, sync::Arc};

use ed25519_dalek::SigningKey;

#[derive(Clone, Debug)]
pub struct Binding {
    pub local_path: String,
    pub remote_address: String,
    pub remote_repository_name: String,
    pub user: Arc<User>,
}

#[derive(Clone, Debug)]
pub struct User {
    pub remote_username: String,
    pub priv_key_path: String,
    pub priv_key: SigningKey,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub users: HashMap<String, Arc<User>>,
    pub bindings: HashMap<String, Arc<Binding>>,
}

pub enum ConfigError {
    FailedToLoadFileForConfig { path: String },
    FailedToLoadLuaLibs,
    LuaErrorForConfig { message: String },
    LuaUnknownErrorForConfig,
    LuaInvalidUsersValue,
    LuaInvalidBindingsValue,
    LuaInvalidUserRemoteUsernameValue { name: String },
    LuaInvalidUserPrivKeyPathValue { name: String },
}

pub fn load_user(name: &String, v: &Table) -> Result<User, ConfigError> {
    let mut user = User {
        remote_username: String::new(),
        priv_key_path: String::new(),
        priv_key: SigningKey::from_bytes(&[0; 32]),
    };

    user.remote_username = match v.get::<String>("remote_username") {
        Ok(value) => value,
        Err(_) => {
            return Err(ConfigError::LuaInvalidUserRemoteUsernameValue { name: name.clone() });
        }
    };

    user.priv_key_path = match v.get::<String>("priv_key_path") {
        Ok(value) => value,
        Err(_) => {
            return Err(ConfigError::LuaInvalidUserPrivKeyPathValue { name: name.clone() });
        }
    };

    Ok(user)
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

    lua.load_std_libs(StdLib::ALL_SAFE)
        .map_err(|_| vec![ConfigError::FailedToLoadLuaLibs])?;

    chunk.exec().map_err(|err| match err {
        SyntaxError {
            message,
            incomplete_input: _,
        } => vec![ConfigError::LuaErrorForConfig { message }],
        _ => vec![ConfigError::LuaUnknownErrorForConfig],
    })?;

    let glob = lua.globals();

    let users = glob
        .get::<Table>("users")
        .map_err(|_| vec![ConfigError::LuaInvalidUsersValue])?;

    let bindings = glob
        .get::<Table>("bindings")
        .map_err(|_| vec![ConfigError::LuaInvalidBindingsValue])?;

    let mut users_map: HashMap<String, Arc<User>> = HashMap::new();

    let _ = users.for_each(|k: String, v: Table| {
        let user = match load_user(&k, &v) {
            Ok(user) => user,
            Err(error) => {
                errors.push(error);
                return Ok(());
            }
        };

        users_map.insert(k, Arc::new(user));
        Ok(())
    });

    let config = Config {
        users: users_map,
        bindings: HashMap::new(),
    };

    Ok(config)
}

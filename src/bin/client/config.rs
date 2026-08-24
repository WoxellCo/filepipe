use mlua::{Error::*, Lua, StdLib, Table, Value};
use std::{
    collections::HashMap,
    fs::{self, read_to_string},
    io::Read,
    sync::Arc,
};

use ed25519_dalek::{SigningKey, pkcs8::DecodePrivateKey};

#[derive(Clone, Debug)]
pub struct Binding {
    pub local_path: String,
    pub remote_address: String,
    pub remote_repository_name: String,
    pub default_user: Arc<User>,
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

#[derive(Clone, Debug)]
pub enum ConfigError {
    FailedToLoadFileForConfig { path: String },
    FailedToLoadLuaLibs,
    LuaErrorForConfig { message: String },
    LuaUnknownErrorForConfig,
    LuaInvalidUsersValue,
    LuaInvalidBindingsValue,
    LuaInvalidUserRemoteUsernameValue { name: String },
    LuaInvalidUserPrivKeyPathValue { name: String },
    LuaInvalidBindingLocalPathValue { name: String },
    LuaInvalidBindingRemoteRepositoryNameValue { name: String },
    LuaInvalidBindingRemoteAddressValue { name: String },
    LuaInvalidBindingDefaultUserValue { name: String },
    LuaUserNotExistentForBinding { name: String, username: String },
    LuaUserInvalidObjectForBinding { name: String },
    LuaInvalidBindingDefaultUserValueType { name: String },
    FailedToOpenKeyFile { name: String },
    FailedToSerializeSigningKey { name: String },
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

    // mk: i will use from `std` for now, but considering using `tokio`, depends...
    let mut priv_key_file = match fs::File::open(&user.priv_key_path) {
        Ok(f) => f,
        Err(_) => {
            return Err(ConfigError::FailedToOpenKeyFile { name: name.clone() });
        }
    };

    let mut priv_key: String = String::new();
    let _ = priv_key_file.read_to_string(&mut priv_key);

    let priv_key = SigningKey::from_pkcs8_pem(&priv_key);
    let priv_key = match priv_key {
        Ok(key) => key,
        Err(_) => {
            return Err(ConfigError::FailedToSerializeSigningKey { name: name.clone() });
        }
    };

    user.priv_key = priv_key;

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
    let mut bindings_map: HashMap<String, Arc<Binding>> = HashMap::new();

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

    let _ = bindings.for_each(|k: String, v: Table| {
        let local_path = match v.get::<String>("local_path") {
            Ok(value) => value,
            Err(_) => {
                errors.push(ConfigError::LuaInvalidBindingLocalPathValue { name: k });
                return Ok(());
            }
        };

        let remote_repository_name = match v.get::<String>("remote_repository_name") {
            Ok(value) => value,
            Err(_) => {
                errors.push(ConfigError::LuaInvalidBindingRemoteRepositoryNameValue { name: k });
                return Ok(());
            }
        };

        let remote_address = match v.get::<String>("remote_address") {
            Ok(value) => value,
            Err(_) => {
                errors.push(ConfigError::LuaInvalidBindingRemoteAddressValue { name: k });
                return Ok(());
            }
        };

        let default_user: Value = match v.get("default_user") {
            Ok(value) => value,
            Err(_) => {
                errors.push(ConfigError::LuaInvalidBindingDefaultUserValue { name: k });
                return Ok(());
            }
        };

        let default_user = match default_user {
            Value::String(username) => {
                let username = username.to_string_lossy();

                match users_map.get(&username) {
                    Some(user) => user.clone(),
                    None => {
                        errors
                            .push(ConfigError::LuaUserNotExistentForBinding { name: k, username });
                        return Ok(());
                    }
                }
            }
            Value::Table(user) => {
                let user = match load_user(&"<anon>".to_string(), &user) {
                    Ok(user) => user,
                    Err(_) => {
                        // mk: it would be nice to find a way to put the output error in there
                        errors.push(ConfigError::LuaUserInvalidObjectForBinding { name: k });
                        return Ok(());
                    }
                };
                Arc::new(user)
            }
            _ => {
                errors.push(ConfigError::LuaInvalidBindingDefaultUserValueType { name: k });
                return Ok(());
            }
        };

        bindings_map.insert(
            k,
            Arc::new(Binding {
                local_path,
                remote_repository_name,
                remote_address,
                default_user,
            }),
        );

        Ok(())
    });

    let config = Config {
        users: users_map,
        bindings: bindings_map,
    };

    Ok(config)
}

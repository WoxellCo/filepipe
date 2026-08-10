use std::{collections::HashMap, fs::read_to_string, vec};
use mlua::{Error::SyntaxError, Lua, StdLib, Table};
use crate::{filepipe::Repository};

#[derive(Clone, Debug)]
pub struct User {
    pub name: String,
    pub pub_key_path: String
}

#[derive(Clone, Debug)]
pub struct Config {
    pub users: HashMap<String, User>,
    pub repository_list_path: String,
    //pub repositories: Vec<Repository>
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
}

//fn load_repository() -> Result<Repository, ConfigError> {
    
//}

fn load_repositories(path: &String, lua: &Lua, mut errors: &Vec<ConfigError>) -> Result<Vec<Repository>, ConfigError> {
    let content = read_to_string(path)
        .map_err(|_| ConfigError::FailedToLoadFileForRepositoryList { path: path.to_owned() })?;

    let chunk = lua.load(content);
    chunk.exec()
        .map_err(|err| {
            match err {
                SyntaxError { message, incomplete_input: _ } => ConfigError::LuaErrorForRepositoryList { message: message },
                _ => ConfigError::LuaUnknownErrorForRepositoryList
            }
        })?;

    let glob = lua.globals();
    let repositories = glob.get::<Table>("repositories")
        .map_err(|_| ConfigError::FailedToLoadRepositoryList)?;

    //repositories.for_each()

    Ok(vec![])
}

pub fn init_config(path: &String) -> Result<Config, Vec<ConfigError>> {
    let mut errors: Vec<ConfigError> = Vec::new();

    let lua = Lua::new();

    let content = read_to_string(path)
        .map_err(|_| vec![ConfigError::FailedToLoadFileForConfig { path: path.to_owned() }])?;

    let chunk = lua.load(content);
    chunk.exec()
        .map_err(|err| {
            match err {
                SyntaxError { message, incomplete_input: _ } => vec![ConfigError::LuaErrorForConfig { message: message }],
                _ => vec![ConfigError::LuaUnknownErrorForConfig]
            }
        })?;

    lua.load_std_libs(StdLib::ALL_SAFE)
        .map_err(|_| vec![ConfigError::FailedToLoadLuaLibs])?;

    let glob = lua.globals();
    let users = glob.get::<Table>("users")
        .map_err(|_| vec![ConfigError::FailedToLoadUsers])?;

    let mut users_map: HashMap<String, User> = HashMap::new();

    let _ = users.for_each(|k: String, v: Table| {
        let mut user_pub_key_path: Option<String> = None;
        match v.get::<String>("pub_key_path") {
            Ok(user_key) => {
                user_pub_key_path = Some(user_key)
            },
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
        users_map.insert(k.clone(), User{
            name: k,
            pub_key_path: user_pub_key_path
        });
        Ok(())
    });
    
    let repository_list_path = match glob.get::<String>("repository_list") {
        Ok(path) => path,
        Err(_) => {
            errors.push(ConfigError::NoRepositoryList);
            return Err(errors);
        }
    };

    /*let repositories = match load_repositories(&repository_list_path, &lua, &errors) {
        Ok(list) => list,
        Err(err) => {
            errors.push(err);
            return Err(errors);
        }
    };*/

    if errors.len() > 0 {
        return Err(errors);
    }

    let config = Config{
        users: users_map,
        repository_list_path,
        //repositories: repositories
    };
    
    Ok(config)
}
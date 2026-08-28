use std::collections::HashMap;
use tokio::time::Instant;

use crate::aio::extract_path_dir_and_name;

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

#[derive(Default, Debug, Clone)]
pub struct RepositoryFile {
    pub path_dir: String,
    pub name: String,
    pub size: u64,
    pub hash: String,
}

#[derive(Clone, PartialEq, Debug)]
pub enum StreamType {
    DownStream,
    UpStream,
}

pub enum ErrorKind {
    InvalidType,
    InvalidRange,
}

pub fn pack_repository_files_info(repository_files: Vec<RepositoryFile>) -> String {
    let mut out: String = String::new();

    for file in repository_files {
        out.push_str(&file.path_dir);
        out.push('/');
        out.push_str(&file.name);
        out.push(':');
        out.push_str(&file.size.to_string());
        out.push(':');
        out.push_str(&file.hash);
        out.push(';');
    }

    out.pop();
    out
}

pub struct UnpackError {
    pub index: usize,
    pub tuple_position: u8,
    pub error_kind: ErrorKind,
}

pub fn unpack_repository_files_info(data: &str) -> Result<Vec<RepositoryFile>, UnpackError> {
    let raw_files_data = data.split(";");
    let mut files: Vec<RepositoryFile> = Vec::new();

    for (index, raw_file_data) in raw_files_data.enumerate() {
        // mk: (full path, size in bytes (string), hash)
        let mut data_tuple: (Option<&str>, Option<&str>, Option<&str>) = (None, None, None);
        let file_data = raw_file_data.split(":");

        for (data_type, file_data_cell) in file_data.enumerate() {
            match data_type {
                0 => {
                    data_tuple.0 = Some(file_data_cell);
                }
                1 => {
                    data_tuple.1 = Some(file_data_cell);
                }
                2 => {
                    data_tuple.2 = Some(file_data_cell);
                }
                _ => {}
            }
        }

        let mut file: RepositoryFile = RepositoryFile::default();

        match data_tuple.0 {
            Some(data) => {
                let path = extract_path_dir_and_name(data);
                file.path_dir = path.0;
                file.name = path.1;
            }
            None => {
                /*return (
                    StatusCode::BAD_REQUEST,
                    headers_out,
                    json!({"error": format!("expected first element for path, index: {index}")})
                        .to_string(),
                );*/
                //break;
                return Ok(files);
            }
        }

        match data_tuple.1 {
            Some(data) => {
                let file_size = data.trim().parse();
                match file_size {
                    Ok(file_size) => {
                        file.size = file_size;
                    }
                    Err(_) => {
                        return Err(UnpackError {
                            index,
                            tuple_position: 1,
                            error_kind: ErrorKind::InvalidRange,
                        });
                    }
                }
            }
            None => {
                return Err(UnpackError {
                    index,
                    tuple_position: 1,
                    error_kind: ErrorKind::InvalidType,
                });
            }
        }

        match data_tuple.2 {
            Some(data) => {
                file.hash = data.to_string();
            }
            None => {
                return Err(UnpackError {
                    index,
                    tuple_position: 2,
                    error_kind: ErrorKind::InvalidType,
                });
            }
        }

        files.push(file);
    }

    Ok(files)
}

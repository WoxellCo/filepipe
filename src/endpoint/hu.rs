// head for upload

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
};
use serde::Deserialize;
use serde_json::json;

use super::AppState;
use crate::filepipe::{RepositoryFile, extract_path_dir_and_name};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostReq {
    pub timestamp: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PutReq {
    pub files: String,
}

// new upload stream request
pub async fn post(
    State(state): State<super::AppState>,
    Json(payload): Json<PostReq>,
) -> (StatusCode, HeaderMap, String) {
    let mut header_out = HeaderMap::new();

    (StatusCode::OK, header_out, String::from(""))
}

// once the server accepts the upload stream request, the client should send the files information and their size with the hash to initialize the writing process
pub async fn put(
    State(mut state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PutReq>,
) -> (StatusCode, HeaderMap, String) {
    let mut headers_out = HeaderMap::new();
    headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let raw_files_data = payload.files.split(";");
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
                break;
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
                        return (
                            StatusCode::BAD_REQUEST,
                            headers_out,
                            json!({"error": format!("invalid second element for size in bytes, and unsigned number is expected, index: {index}")})
                                .to_string(),
                        );
                    }
                }
            }
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    headers_out,
                    json!({"error": format!("expected second element for size in bytes, index: {index}")})
                        .to_string(),
                );
            }
        }

        match data_tuple.2 {
            Some(data) => {
                file.hash = data.to_string();
            }
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    headers_out,
                    json!({"error": format!("expected third element for hash, index: {index}")})
                        .to_string(),
                );
            }
        }

        files.push(file);
    }

    todo!("actually initialize the stream and update the app state");

    (StatusCode::OK, headers_out, String::new())
}

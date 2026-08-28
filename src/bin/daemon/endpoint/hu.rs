// head for upload

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
};
use serde::Deserialize;
use serde_json::json;

use crate::endpoint::Expirable;

use super::AppState;
use filepipe::{
    aio::extract_path_dir_and_name,
    filepipe::{RepositoryFile, StreamType, unpack_repository_files_info},
};

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
    headers: HeaderMap,
    Path(name): Path<String>,
    //Json(payload): Json<PostReq>,
) -> (StatusCode, HeaderMap, String) {
    let mut headers_out = HeaderMap::new();
    headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let auth = headers.get("authorization");

    let access = match state.authenticate(auth, &name).await {
        Some(access) => access,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                headers_out,
                json!({
                    "error": "invalid repository or invalid or missing access key"
                })
                .to_string(),
            );
        }
    };

    if !access.content.write {
        return (
            StatusCode::FORBIDDEN,
            headers_out,
            json!({"error": "user doesn't have enough permissions to perform this action"})
                .to_string(),
        );
    }

    let new_session_key = match state.register_upstream_session(name.as_str()).await {
        Ok(key) => key,
        Err(()) => {
            return (
                StatusCode::FORBIDDEN,
                headers_out,
                json!({"error": "there is already an active upstream session for this repository"})
                    .to_string(),
            );
        }
    };

    (
        StatusCode::OK,
        headers_out,
        json!({"key": new_session_key}).to_string(),
    )
}

// once the server accepts the upload stream request, the client should send the files information and their size with the hash to initialize the writing process
pub async fn put(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<PutReq>,
) -> (StatusCode, HeaderMap, String) {
    let session = state
        .get_session_by_key_in_header_value(headers.get("authorization"))
        .await;

    let mut headers_out = HeaderMap::new();
    headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let key = match session.0 {
        Some(key) => key,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                headers_out,
                json!({"error": "invalid or missing session key"}).to_string(),
            );
        }
    };

    let mut session = match session.1 {
        Some(session) => session,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                headers_out,
                json!({"error": "invalid or missing session key"}).to_string(),
            );
        }
    };

    if session.is_expired() {
        return (
            StatusCode::GONE,
            headers_out,
            json!({"error": "session expired"}).to_string(),
        );
    }

    if session.stream_type != StreamType::UpStream {
        return (
            StatusCode::FORBIDDEN,
            headers_out,
            json!({"error": format!("invalid stream type")}).to_string(),
        );
    }

    let files = match unpack_repository_files_info(&payload.files) {
        Ok(files) => files,
        Err(error) => match error.tuple_position {
            1 => match error.error_kind {
                filepipe::filepipe::ErrorKind::InvalidRange => return (
                    StatusCode::BAD_REQUEST,
                    headers_out,
                    json!({"error": format!("invalid second element for size in bytes, and unsigned number is expected, index: {}", error.index)})
                        .to_string(),
                    ),
                filepipe::filepipe::ErrorKind::InvalidType => return (
                    StatusCode::BAD_REQUEST,
                    headers_out,
                    json!({"error": format!("expected second element for size in bytes, index: {}", error.index)})
                        .to_string(),
                    ),
            },
            2 => match error.error_kind {
                filepipe::filepipe::ErrorKind::InvalidType => return (
                    StatusCode::BAD_REQUEST,
                    headers_out,
                    json!({"error": format!("expected third element for hash, index: {}", error.index)})
                        .to_string(),
                    ),
                filepipe::filepipe::ErrorKind::InvalidRange => return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    headers_out,
                    json!({"error": format!("unknown, index: {}", error.index)})
                        .to_string(),
                    ),
            },
            _ => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    headers_out,
                    json!({"error": format!("unknown, index: {}", error.index)})
                        .to_string(),
                    );
            }
        }
    };

    println!("{:?}", files);
    //todo!("actually initialize the stream and update the app state");

    state
        .with_session_mut(&key, |session| {
            session.update_last_activity();
            session.file_list = files;
        })
        .await;

    (StatusCode::OK, headers_out, String::new())
}

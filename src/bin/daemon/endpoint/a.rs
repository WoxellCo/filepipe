// authentication process

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;

use {
    crate::endpoint::{AccessKey, AppState},
    filepipe::key_gen::{self, verify_signature},
};

pub async fn post(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> (StatusCode, HeaderMap, String) {
    let mut headers_out = HeaderMap::new();
    headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    let user = state.config.users.get(&username);
    let user = match user {
        Some(user) => user.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                headers_out,
                json!({"error": "invalid user"}).to_string(),
            );
        }
    };

    let mut access_keys = state.access_keys.write().await;
    let mut key;
    loop {
        key = key_gen::generate_random_string(16);
        if !access_keys.contains_key(&key) {
            break;
        }
    }
    access_keys.insert(
        key.clone(),
        AccessKey {
            user,
            signed: false,
            expire: Utc::now() + chrono::Duration::minutes(5),
        },
    );
    (
        StatusCode::OK,
        headers_out,
        json!({
            "accessKey": key
        })
        .to_string(),
    )
}

/*#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutReq {
    sign: String,
}*/

pub async fn put(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> (StatusCode, HeaderMap, String) {
    let headers_out = HeaderMap::new();

    let key = match headers.get("authorization") {
        Some(key) => match key.to_str() {
            Ok(key) => key,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    headers_out,
                    String::new(),
                );
            }
        },
        None => {
            return (StatusCode::UNAUTHORIZED, headers_out, String::new());
        }
    };

    let access_keys = state.access_keys.write().await;
    let access_key = match access_keys.get(key) {
        Some(key) => key,
        None => {
            return (StatusCode::UNAUTHORIZED, headers_out, String::new());
        }
    };

    let signed_key: [u8; 64] = match body.as_ref().try_into() {
        Ok(b) => b,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, headers_out, String::new());
        }
    };

    let user = access_key.user.clone();

    match verify_signature(&user.pub_key, key.as_bytes(), &signed_key) {
        Ok(()) => (StatusCode::OK, headers_out, String::new()),
        Err(_) => (StatusCode::FORBIDDEN, headers_out, String::new()),
    }
}

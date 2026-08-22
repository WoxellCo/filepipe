// info

use axum::http::{HeaderMap, StatusCode};

pub async fn get() -> (StatusCode, HeaderMap, String) {
    let mut header_out = HeaderMap::new();

    (StatusCode::OK, header_out, String::from(""))
}

pub async fn post() -> (StatusCode, HeaderMap, String) {
    let mut header_out = HeaderMap::new();

    (StatusCode::OK, header_out, String::from(""))
}
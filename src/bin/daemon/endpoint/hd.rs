// head for download

use axum::{Json, extract::State, http::{HeaderMap, StatusCode}};
use serde::Deserialize;

async fn post() -> (StatusCode, HeaderMap, String) {
    let mut header_out = HeaderMap::new();

    (StatusCode::OK, header_out, String::from(""))
}
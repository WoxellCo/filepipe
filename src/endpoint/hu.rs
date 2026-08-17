// head for upload

use axum::{Json, extract::State, http::{HeaderMap, StatusCode}};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadHeadPostReq {
    pub timestamp: String
}

// new upload stream request
pub async fn post(State(state): State<super::AppState>, Json(payload): Json<UploadHeadPostReq>) -> (StatusCode, HeaderMap, String) {
    let mut header_out = HeaderMap::new();

    (StatusCode::OK, header_out, String::from(""))
}
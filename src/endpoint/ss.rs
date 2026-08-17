// stream (file transfer)

use std::{format, ops::Bound, path::{Component, PathBuf}, sync::Arc};

use axum::{body::Body, extract::{Path, Query, State}, http::{HeaderMap, StatusCode, header::CONTENT_TYPE}};
//use serde::Deserialize;
use serde_json::json;
//use axum_range::{Ranged, KnownSize};
use axum_extra::{TypedHeader, headers::Range};

use crate::endpoint::{AppState, Session, StreamType};
use crate::aio::read_chunk;

/*
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownStreamReq {
    //pub path: String,
    pub cursor: u64,
    pub chunk_size: usize
}*/

// mk: todo: implement a function to sanitize the path and prevent repository escaping
//           it's an important vulnerability but not critical (in our cases) since it requires authentication

pub async fn get(
    State(mut state): State<AppState>,
    Path((name, path)): Path<(String, String)>,
    headers: HeaderMap,
    //Query(query): Query<DownStreamReq>,
    range: Option<TypedHeader<Range>>
) -> (StatusCode, HeaderMap, Body) {
    let mut headers_out = HeaderMap::new();
    let auth = headers.get("authorization");

    // TEST ONLY //
    state.sessions.insert(String::from("123"), Session {
        stream_type: StreamType::DownStream,
        repository: Arc::new(state.config.repositories.get("repository-name").unwrap().clone())
    });
    ///////////////

    let session = match state.get_session_by_key_in_header_value(auth) {
        Some(session) => session,
        None => {
            headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());
            return (StatusCode::UNAUTHORIZED, headers_out, Body::from(json!({ "error": "invalid or missing session key" }).to_string()));
        }
    };

    if session.stream_type != StreamType::DownStream {
        headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        return (StatusCode::FORBIDDEN, headers_out, Body::from(json!({ "error": "invalid stream type" }).to_string()));
    }

    if session.repository.name != name {
        headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        return (StatusCode::FORBIDDEN, headers_out, Body::from(json!({ "error": "unmatched repository" }).to_string()));
    }

    let full_path = format!("{}/{}", session.repository.path, path);
    let metadata = match tokio::fs::metadata(&full_path).await {
        Ok(m) => m,
        Err(_) => {
            headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());
            return (StatusCode::NOT_FOUND, headers_out, Body::from(json!({ "error": "resource not found" }).to_string()));
        },
    };
    let file_size = metadata.len();
    let had_range_header = range.is_some();

    let (begin, end) = match range {
        Some(TypedHeader(range)) => {
            match range.satisfiable_ranges(file_size).next() {
                Some((Bound::Included(start), Bound::Included(end))) => (start, end),
                Some((Bound::Included(start), Bound::Unbounded)) => (start, file_size - 1),
                Some((Bound::Unbounded, Bound::Included(suffix_len))) => {
                    (file_size.saturating_sub(suffix_len), file_size - 1)
                }
                _ => {
                    headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());
                    return (StatusCode::RANGE_NOT_SATISFIABLE, headers_out, Body::from(json!({ "error": "invalid range" }).to_string()));
                }
            }
        }
        None => (0, file_size - 1),
    };
    let chunk = read_chunk(&full_path, begin, (end - begin + 1) as usize).await;

    match chunk {
        Ok(bytes) => {
            headers_out.insert(CONTENT_TYPE, "application/octet-stream".parse().unwrap());
            headers_out.insert(axum::http::header::CONTENT_RANGE, format!("bytes {begin}-{end}/{file_size}").parse().unwrap());
            let status = if had_range_header { StatusCode::PARTIAL_CONTENT } else { StatusCode::OK };
            (status, headers_out, Body::from(bytes))
        },
        Err(_) => {
            headers_out.insert(CONTENT_TYPE, "application/json".parse().unwrap());
            return (StatusCode::INTERNAL_SERVER_ERROR, headers_out, Body::from(json!({ "error": "failed to access file in read mode" }).to_string()));
        },
    }
}
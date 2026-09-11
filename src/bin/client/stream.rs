use axum::http::{HeaderMap, HeaderValue};
use filepipe::{
    aio::{self, compose_path_dir_and_name},
    filepipe::RepositoryFile,
};
use reqwest::header::{AUTHORIZATION, RANGE};
use std::{collections::HashMap, sync::Arc};
use tokio::{fs, sync::RwLock};

use crate::caller::ClientState;

pub struct FileStream {
    pub file_entry: RepositoryFile,
    pub status: Arc<RwLock<FileStreamStatus>>,
}

#[derive(Clone)]
pub struct FileStreamStatus {
    total_bytes: u64,
    processed_bytes: u64,
}

impl FileStream {
    pub async fn retrieve_progress_status(&self) -> FileStreamStatus {
        let status = self.status.read().await;
        status.clone()
    }

    pub async fn retrieve_progress_status_percentage(&self) -> f32 {
        let status = self.retrieve_progress_status().await;
        status.processed_bytes as f32 * 100f32 / status.total_bytes as f32
    }
}

impl ClientState {
    pub async fn upstream_files<F>(
        &self,
        file_entries: &Vec<RepositoryFile>,
        file_temps: &HashMap<String, String>,
        max_concurrent_requests: u32,
        cycle_output: F,
    ) where
        F: AsyncFn(&FileStream),
    {
        let Some(session_key) = &self.session_key else {
            println!("err:a");
            return; // todo: handle error
        };

        println!("{:?}", file_entries);

        let local_repository_path = &self.current_binding.local_path;

        for file_entry in file_entries.iter() {
            let path = compose_path_dir_and_name(&file_entry.path_dir, &file_entry.name);
            let og_path = match file_temps.get(&path) {
                Some(temp) => temp,
                None => &path,
            };
            let Ok(mut file) =
                fs::File::open(format!("{}/{}", local_repository_path, og_path)).await
            else {
                println!("e:1");
                return; // todo: error
            };

            for cursor in (0..file_entry.size).step_by(262144) {
                println!("bbb");
                let chunk_len = 262144.min(file_entry.size - cursor);
                let mut headers = HeaderMap::new();
                headers.insert(AUTHORIZATION, HeaderValue::from_str(session_key).unwrap());
                headers.insert(
                    RANGE,
                    HeaderValue::from_str(&format!("bytes={}-{}", cursor, cursor + chunk_len))
                        .unwrap(),
                );

                let Ok(bytes) =
                    aio::read_chunk_from_open_stream(&mut file, cursor, chunk_len as usize).await
                else {
                    println!("e:2");
                    return; // todo: error as always
                };

                let response = self
                    .client
                    .post(format!(
                        "{}/ss/{}/{}",
                        self.current_binding.remote_address,
                        self.current_binding.remote_repository_name,
                        compose_path_dir_and_name(&file_entry.path_dir, &file_entry.name)
                    ))
                    .headers(headers)
                    .body(bytes)
                    .send()
                    .await;

                //println!("{:?}", response);
            }
        }

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(session_key).unwrap());
        let response = self
            .client
            .put(format!("{}/ss", self.current_binding.remote_address))
            .headers(headers)
            .send()
            .await;

        println!("resp: {:?}", response);
    }

    async fn upstream_file(&self, file_stream: &FileStream) {}
}

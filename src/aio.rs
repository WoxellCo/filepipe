use ignore::WalkBuilder;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use xxhash_rust::xxh3::Xxh3;

use crate::filepipe::RepositoryFile;

#[derive(Debug, Clone)]
pub enum IOError {
    FailedToOpenFile { path: String },
    FailedToConvertHashBytesToString { path: String },
}

pub async fn read_chunk(
    //state: Option<&AppState>,
    path: &str,
    offset: u64,
    size: usize,
) -> std::io::Result<Vec<u8>> {
    /*if let state = Some(state) {
        todo!("implement!!!");
    }*/

    let mut file = File::open(path).await?;
    file.seek(SeekFrom::Start(offset)).await?;

    let mut buffer: Vec<u8> = vec![0u8; size];
    file.read_exact(&mut buffer).await?;
    Ok(buffer)
}

pub fn extract_path_dir_and_name(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((dir, name)) => (dir.to_owned(), name.to_owned()),
        None => (String::new(), path.to_owned()),
    }
}

async fn hash_file_streaming(path: &str) -> std::io::Result<u128> {
    let mut file = File::open(path).await?;
    let mut hasher = Xxh3::new();
    let mut buf = vec![0u8; 64 * 1024];

    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }

    Ok(hasher.digest128())
}

pub async fn get_file_list_in_dir_with_fpignore(
    path_dir: &str,
) -> Result<Vec<RepositoryFile>, IOError> {
    let mut builder = WalkBuilder::new(path_dir);
    builder
        .standard_filters(false)
        .add_custom_ignore_filename(".fpignore")
        .filter_entry(|entry| entry.file_name() != ".fp");
    //.current_dir(path_dir);

    let root = Path::new(path_dir);

    let mut entries: Vec<RepositoryFile> = Vec::new();

    // mk: yeah, i have to improve some things here: better error handling and push the entry struct directly without making it a mutable varibale first
    for entry in builder.build().flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_dir() || metadata.is_symlink() {
                continue;
            }

            let mut repository_file = RepositoryFile {
                size: metadata.len(),
                path_dir: String::new(),
                hash: String::new(),
                name: String::new(),
            };

            let full_path = entry.path();
            let file_path = match full_path.strip_prefix(root).unwrap().to_str() {
                Some(path) => path.to_string(),
                None => continue,
            };

            let full_path = match full_path.to_str() {
                Some(path) => path.to_string(),
                None => continue,
            };

            (repository_file.path_dir, repository_file.name) =
                extract_path_dir_and_name(&file_path);

            let hash: [u8; 16] = hash_file_streaming(&full_path)
                .await
                .map_err(|_| IOError::FailedToOpenFile {
                    path: file_path.clone(),
                })?
                .to_le_bytes();

            repository_file.hash = hex::encode(hash);

            entries.push(repository_file);
        }
    }

    Ok(entries)
}

use std::io::SeekFrom;
use std::path::PathBuf;
use ignore::WalkBuilder;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use crate::filepipe::RepositoryFile;

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

pub async fn get_file_list_in_dir_with_fpignore(path_dir: &str) -> Vec<RepositoryFile> {
    let mut builder = WalkBuilder::new(path_dir);
    builder
        .standard_filters(false)
        .add_custom_ignore_filename(".fpignore")
        .filter_entry(|entry| {
            entry.file_name() != ".fp"
        });

    let mut paths: Vec<RepositoryFile> = Vec::new();

    for entry in builder.build() {
        match entry {
            Ok(entry) => {
                //paths.push(entry.path().to_path_buf());
                match entry.metadata() {
                    Ok(metadata) => {
                        if metadata.is_dir() || metadata.is_symlink() {
                            continue;
                        }
                        let mut repository_file = RepositoryFile {
                            size: metadata.len(),
                            path_dir: String::new(),
                            hash: String::new(),
                            name: String::new(),
                        };

                        let full_path = match entry.path().to_str() {
                            Some(path) => path.to_string(),
                            None => continue
                        };

                        (repository_file.path_dir, repository_file.name) = extract_path_dir_and_name(&full_path);

                        //xxh3::
                        todo!("hash files");
                    },
                    Err(_) => {}
                }
                //println!("{}", entry.path().display());
                //entry.path_is_symlink()
            }
            Err(err) => eprintln!("error: {err}"),
        }
    }

    paths
}

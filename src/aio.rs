use ignore::WalkBuilder;
use std::collections::HashMap;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use xxhash_rust::xxh3::Xxh3;

use crate::filepipe::RepositoryFile;
use crate::filepipe::transf::{FileTranformations, TempPaths};

#[derive(Debug, Clone)]
pub enum IOError {
    FailedToOpenFile { path: String },
    FailedToConvertHashBytesToString { path: String },
    FailedToConvertHashStringToBytes { hash: String },
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

pub async fn read_chunk_from_open_stream(
    //state: Option<&AppState>,
    file: &mut File,
    offset: u64,
    size: usize,
) -> std::io::Result<Vec<u8>> {
    /*if let state = Some(state) {
        todo!("implement!!!");
    }*/

    file.seek(SeekFrom::Start(offset)).await?;

    let mut buffer: Vec<u8> = vec![0u8; size];
    file.read_exact(&mut buffer).await?;
    Ok(buffer)
}

pub async fn write_chunk(path: &str, offset: u64, data: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(path).await?;
    file.seek(SeekFrom::Start(offset)).await?;
    file.write_all(data).await?;
    Ok(())
}

pub fn extract_path_dir_and_name(path: &str) -> (String, String) {
    match path.rsplit_once('/') {
        Some((dir, name)) => (dir.to_owned(), name.to_owned()),
        None => (String::new(), path.to_owned()),
    }
}

pub fn compose_path_dir_and_name(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        return name.to_string();
    }
    format!("{dir}/{name}")
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
) -> Result<HashMap<String, RepositoryFile>, IOError> {
    let mut builder = WalkBuilder::new(path_dir);
    builder
        .standard_filters(false)
        .add_custom_ignore_filename(".fpignore")
        .filter_entry(|entry| entry.file_name() != ".fp");
    //.current_dir(path_dir);

    let root = Path::new(path_dir);

    let mut entries: HashMap<String, RepositoryFile> = HashMap::new();

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

            entries.insert(file_path, repository_file);
        }
    }

    Ok(entries)
}

pub fn hash_str_to_u128(hash_str: &str) -> Result<u128, ()> {
    let hash = hex::decode(hash_str).map_err(|_| ())?;

    let hash: [u8; 16] = match hash.as_array() {
        Some(hash) => *hash,
        None => {
            return Err(());
        }
    };

    let hash: u128 = u128::from_le_bytes(hash);

    Ok(hash)
}

pub async fn create_file_with_size(path: &str, size: u64) -> Result<(), ()> {
    let (path_dir, _) = extract_path_dir_and_name(path);
    let _ = tokio::fs::create_dir_all(path_dir).await;

    let file = match File::create(path).await {
        Ok(file) => file,
        Err(e) => {
            println!("_ {:?}", e);
            return Err(());
        }
    };

    if let Err(error) = file.set_len(size).await {
        println!("e {:?}", error);
        return Err(());
    };

    Ok(())
}

pub async fn extract_hash_file_sizes_from_file_entries(
    entries: &Vec<RepositoryFile>,
) -> HashMap<u128, u64> {
    let mut out = HashMap::with_capacity(entries.len());
    for entry in entries {
        let Ok(hash) = hash_str_to_u128(&entry.hash) else {
            continue;
        };
        out.insert(hash, entry.size);
    }

    out
}

pub async fn allocate_network_disk_from_transf(
    transformations: &FileTranformations,
    hash_sizes: &HashMap<u128, u64>,
    repository_key: &str,
) {
    for v in &transformations.to_transfer {
        let Some(origin) = v.1.first() else {
            println!("aa");
            continue;
        };

        let Some(size) = hash_sizes.get(v.0) else {
            println!("bb");
            continue;
        };

        println!(
            "{:?}",
            create_file_with_size(&format!(".fp/nt/{repository_key}/{origin}"), *size).await
        );
    }
}

pub async fn execute_transf_fs(
    transformations: &FileTranformations,
    temp_paths: &TempPaths,
    repository_path: &str,
    repository_key: &str,
) {
    for t in &transformations.to_move {
        tokio::fs::rename(
            format!("{repository_path}/{}", t.0),
            format!("{repository_path}/{}", t.1),
        )
        .await;
    }

    for t in &transformations.to_copy {
        let mut origin = t.0;

        if let Some(m) = &t.1.0 {
            let path = extract_path_dir_and_name(m);
            println!("{repository_path}/{origin}");
            println!("1: {:?}", tokio::fs::create_dir_all(&path.0).await);
            println!(
                "2: {:?}",
                tokio::fs::rename(
                    format!("{repository_path}/{origin}"),
                    format!("{repository_path}/{m}")
                )
                .await
            );
            origin = m;
        }

        for c in &t.1.1 {
            let path = extract_path_dir_and_name(c);
            println!("{repository_path}/{origin}");
            println!("3: {:?}", tokio::fs::create_dir_all(&path.0).await);
            println!(
                "4: {:?}",
                tokio::fs::copy(
                    format!("{repository_path}/{origin}"),
                    format!("{repository_path}/{c}")
                )
                .await
            );
        }
    }

    for v in &transformations.to_transfer {
        let Some(origin) = v.1.first() else {
            continue;
        };
        let nt_path = format!(".fp/nt/{repository_key}/{origin}");

        for e in v.1 {
            if origin == e {
                continue;
            }
            let dest_path = format!("{repository_path}/{e}");
            let path = extract_path_dir_and_name(&dest_path);
            println!("DEST_PATH: {dest_path}");
            tokio::fs::create_dir_all(&path.0).await;
            println!("5: {:?}", tokio::fs::copy(&nt_path, &dest_path).await);
        }
        let dest_path = format!("{repository_path}/{origin}");
        println!("DEST_PATH (OUT): {dest_path}");
        let path = extract_path_dir_and_name(&dest_path);
        tokio::fs::create_dir_all(&path.0).await;
        tokio::fs::rename(nt_path, &dest_path).await;
    }

    for d in &transformations.to_delete {
        tokio::fs::remove_file(format!("{repository_path}/{d}")).await;
    }

    for tp in temp_paths {
        println!(
            "tp.0: {} | tp.1: {}",
            format!("{repository_path}/{}", tp.0),
            format!("{repository_path}/{}", tp.1)
        );
        tokio::fs::rename(
            format!("{repository_path}/{}", tp.0),
            format!("{repository_path}/{}", tp.1),
        )
        .await;
    }
}

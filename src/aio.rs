use std::io::SeekFrom;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt};


pub async fn read_chunk(
    //state: Option<&AppState>,
    path: &str,
    offset: u64,
    size: usize
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
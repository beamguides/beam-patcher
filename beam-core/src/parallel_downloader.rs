use crate::{Config, Error, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

const CHUNK_SIZE: u64 = 1024 * 1024 * 2;
const MAX_PARALLEL_CHUNKS: usize = 4;

pub struct ParallelDownloader {
    client: Client,
    #[allow(dead_code)]
    config: Config,
    max_parallel: usize,
}

#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub percentage: f32,
}

impl ParallelDownloader {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Beam-Patcher/1.0")
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        
        Ok(ParallelDownloader {
            client,
            config,
            max_parallel: MAX_PARALLEL_CHUNKS,
        })
    }
    
    pub async fn download_file_parallel(
        &self,
        url: &str,
        destination: &Path,
    ) -> Result<PathBuf> {
        info!("Starting parallel download: {}", url);
        
        let head_response = self.client.head(url).send().await?;
        
        if !head_response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "HTTP error: {}",
                head_response.status()
            )));
        }
        
        let total_size = head_response
            .content_length()
            .ok_or_else(|| Error::DownloadFailed("Content-Length header missing".to_string()))?;
        
        let supports_range = head_response
            .headers()
            .get("accept-ranges")
            .map(|v| v.to_str().unwrap_or("") == "bytes")
            .unwrap_or(false);
        
        if !supports_range || total_size < CHUNK_SIZE {
            info!("Server doesn't support range requests or file too small, using single-threaded download");
            return self.download_single_threaded(url, destination, total_size).await;
        }
        
        info!("Downloading {} bytes in parallel chunks", total_size);
        
        tokio::fs::create_dir_all(destination.parent().unwrap()).await?;
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(destination)
            .await?;
        file.set_len(total_size).await?;
        drop(file);
        
        let semaphore = Arc::new(Semaphore::new(self.max_parallel));
        let mut futures = FuturesUnordered::new();
        
        let num_chunks = (total_size + CHUNK_SIZE - 1) / CHUNK_SIZE;
        
        for chunk_index in 0..num_chunks {
            let start = chunk_index * CHUNK_SIZE;
            let end = std::cmp::min(start + CHUNK_SIZE - 1, total_size - 1);
            
            let client = self.client.clone();
            let url = url.to_string();
            let destination = destination.to_path_buf();
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            
            futures.push(tokio::spawn(async move {
                let result = download_chunk(&client, &url, &destination, start, end, chunk_index).await;
                drop(permit);
                result
            }));
        }
        
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    warn!("Chunk download failed: {}", e);
                    return Err(e);
                }
                Err(e) => {
                    warn!("Task join error: {}", e);
                    return Err(Error::DownloadFailed(format!("Task join error: {}", e)));
                }
            }
        }
        
        info!("Parallel download completed: {:?}", destination);
        Ok(destination.to_path_buf())
    }
    
    async fn download_single_threaded(
        &self,
        url: &str,
        destination: &Path,
        total_size: u64,
    ) -> Result<PathBuf> {
        debug!("Single-threaded download: {}", url);
        
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "HTTP error: {}",
                response.status()
            )));
        }
        
        tokio::fs::create_dir_all(destination.parent().unwrap()).await?;
        let mut file = File::create(&destination).await?;
        
        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            
            if downloaded % (1024 * 1024 * 10) == 0 || downloaded == total_size {
                let percentage = (downloaded as f32 / total_size as f32) * 100.0;
                debug!("Downloaded: {:.1}%", percentage);
            }
        }
        
        file.flush().await?;
        
        info!("Download completed: {:?}", destination);
        Ok(destination.to_path_buf())
    }
    
    pub async fn resume_download(
        &self,
        url: &str,
        destination: &Path,
    ) -> Result<PathBuf> {
        if !destination.exists() {
            return self.download_file_parallel(url, destination).await;
        }
        
        let current_size = tokio::fs::metadata(destination).await?.len();
        
        let head_response = self.client.head(url).send().await?;
        let total_size = head_response
            .content_length()
            .ok_or_else(|| Error::DownloadFailed("Content-Length header missing".to_string()))?;
        
        if current_size >= total_size {
            info!("File already downloaded completely");
            return Ok(destination.to_path_buf());
        }
        
        info!(
            "Resuming download from {} / {} bytes",
            current_size, total_size
        );
        
        let response = self
            .client
            .get(url)
            .header("Range", format!("bytes={}-", current_size))
            .send()
            .await?;
        
        if !response.status().is_success() && response.status().as_u16() != 206 {
            return Err(Error::DownloadFailed(format!(
                "HTTP error: {}",
                response.status()
            )));
        }
        
        let mut file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(destination)
            .await?;
        
        let mut stream = response.bytes_stream();
        let mut downloaded = current_size;
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            
            if downloaded % (1024 * 1024 * 10) == 0 || downloaded == total_size {
                let percentage = (downloaded as f32 / total_size as f32) * 100.0;
                debug!("Downloaded: {:.1}%", percentage);
            }
        }
        
        file.flush().await?;
        
        info!("Resume download completed: {:?}", destination);
        Ok(destination.to_path_buf())
    }
}

async fn download_chunk(
    client: &Client,
    url: &str,
    destination: &Path,
    start: u64,
    end: u64,
    chunk_index: u64,
) -> Result<()> {
    debug!(
        "Downloading chunk {}: bytes {}-{}",
        chunk_index, start, end
    );
    
    let response = client
        .get(url)
        .header("Range", format!("bytes={}-{}", start, end))
        .send()
        .await?;
    
    if !response.status().is_success() && response.status().as_u16() != 206 {
        return Err(Error::DownloadFailed(format!(
            "HTTP error: {}",
            response.status()
        )));
    }
    
    let bytes = response.bytes().await?;
    
    let mut file = OpenOptions::new()
        .write(true)
        .open(destination)
        .await?;
    
    file.seek(std::io::SeekFrom::Start(start)).await?;
    file.write_all(&bytes).await?;
    file.flush().await?;
    
    debug!("Chunk {} downloaded successfully", chunk_index);
    
    Ok(())
}

use crate::{Config, Error, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use tokio::sync::{Mutex, Semaphore};
use tracing::{debug, info, warn};

const CHUNK_SIZE: u64 = 1024 * 1024 * 2;
const MAX_PARALLEL_CHUNKS: usize = 4;
const CHUNK_RETRY_ATTEMPTS: usize = 3;

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

        // All writes target a single .part file, renamed to destination atomically
        // at the end. Failure halfway leaves only the .part, not a half-written
        // "destination" that callers might mistakenly treat as complete.
        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let tmp_path = tmp_path_for(destination);

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .await?;
        file.set_len(total_size).await?;
        // Share the file handle across chunk workers instead of reopening per chunk.
        let shared_file = Arc::new(Mutex::new(file));

        let semaphore = Arc::new(Semaphore::new(self.max_parallel));
        let mut futures = FuturesUnordered::new();

        let num_chunks = total_size.div_ceil(CHUNK_SIZE);

        for chunk_index in 0..num_chunks {
            let start = chunk_index * CHUNK_SIZE;
            let end = std::cmp::min(start + CHUNK_SIZE - 1, total_size - 1);

            let client = self.client.clone();
            let url = url.to_string();
            let file = Arc::clone(&shared_file);
            let permit = semaphore.clone().acquire_owned().await
                .map_err(|e| Error::DownloadFailed(format!("Semaphore closed: {}", e)))?;

            futures.push(tokio::spawn(async move {
                let result = download_chunk(&client, &url, &file, start, end, chunk_index).await;
                drop(permit);
                result
            }));
        }

        let mut first_error: Option<Error> = None;
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(_)) => {}
                Ok(Err(e)) => {
                    if first_error.is_none() {
                        warn!("Chunk download failed: {}", e);
                        first_error = Some(e);
                    }
                    // Cancel any still-running chunk tasks so we don't keep writing
                    // into a file we're about to delete.
                    for f in futures.iter() { f.abort(); }
                }
                Err(e) => {
                    if first_error.is_none() {
                        warn!("Task join error: {}", e);
                        first_error = Some(Error::DownloadFailed(format!("Task join error: {}", e)));
                    }
                    for f in futures.iter() { f.abort(); }
                }
            }
        }

        if let Some(err) = first_error {
            // Drop the file handle and remove the partial download.
            drop(shared_file);
            let _ = tokio::fs::remove_file(&tmp_path).await;
            return Err(err);
        }

        // Flush + sync, then atomic rename.
        {
            let mut guard = shared_file.lock().await;
            guard.flush().await?;
            guard.sync_data().await?;
        }
        drop(shared_file);
        tokio::fs::rename(&tmp_path, destination).await?;

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

        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let tmp_path = tmp_path_for(destination);

        {
            let file = File::create(&tmp_path).await?;
            let mut writer = BufWriter::with_capacity(64 * 1024, file);

            let mut stream = response.bytes_stream();
            let mut downloaded: u64 = 0;
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                writer.write_all(&chunk).await?;
                downloaded += chunk.len() as u64;
                if downloaded.is_multiple_of(1024 * 1024 * 10) || downloaded == total_size {
                    let percentage = (downloaded as f32 / total_size as f32) * 100.0;
                    debug!("Downloaded: {:.1}%", percentage);
                }
            }

            writer.flush().await?;
        }
        tokio::fs::rename(&tmp_path, destination).await?;

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

        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(destination)
            .await?;
        let mut writer = BufWriter::with_capacity(64 * 1024, file);

        let mut stream = response.bytes_stream();
        let mut downloaded = current_size;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            writer.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;

            if downloaded % (1024 * 1024 * 10) == 0 || downloaded == total_size {
                let percentage = (downloaded as f32 / total_size as f32) * 100.0;
                debug!("Downloaded: {:.1}%", percentage);
            }
        }

        writer.flush().await?;

        info!("Resume download completed: {:?}", destination);
        Ok(destination.to_path_buf())
    }
}

async fn download_chunk(
    client: &Client,
    url: &str,
    file: &Arc<Mutex<File>>,
    start: u64,
    end: u64,
    chunk_index: u64,
) -> Result<()> {
    debug!(
        "Downloading chunk {}: bytes {}-{}",
        chunk_index, start, end
    );

    // Retry transient failures (e.g. brief connection drop) before giving up.
    let mut last_err: Option<Error> = None;
    let mut bytes: Option<bytes::Bytes> = None;
    for attempt in 0..CHUNK_RETRY_ATTEMPTS {
        match client
            .get(url)
            .header("Range", format!("bytes={}-{}", start, end))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 206 => {
                match resp.bytes().await {
                    Ok(b) => { bytes = Some(b); break; }
                    Err(e) => last_err = Some(e.into()),
                }
            }
            Ok(resp) => {
                last_err = Some(Error::DownloadFailed(format!("HTTP error: {}", resp.status())));
            }
            Err(e) => last_err = Some(e.into()),
        }
        if attempt + 1 < CHUNK_RETRY_ATTEMPTS {
            let backoff = std::time::Duration::from_millis(200 * (1u64 << attempt));
            tokio::time::sleep(backoff).await;
        }
    }

    let bytes = bytes.ok_or_else(|| last_err.unwrap_or_else(|| Error::DownloadFailed("Chunk download failed".into())))?;

    // Write under lock so concurrent chunks don't race on seek/write.
    let mut guard = file.lock().await;
    guard.seek(std::io::SeekFrom::Start(start)).await?;
    guard.write_all(&bytes).await?;
    // Do NOT flush per chunk — the outer driver flushes/syncs once at the end.

    debug!("Chunk {} downloaded successfully", chunk_index);

    Ok(())
}

fn tmp_path_for(destination: &Path) -> PathBuf {
    let mut s = destination.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

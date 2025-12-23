use crate::{Config, Error, Result};
use futures::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, warn};

pub struct Downloader {
    client: Client,
    config: Config,
}

impl Downloader {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Beam-Patcher/1.0")
            .connect_timeout(std::time::Duration::from_secs(30))
            .timeout(std::time::Duration::from_secs(300))
            .tcp_keepalive(std::time::Duration::from_secs(60))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()?;
        
        Ok(Downloader { client, config })
    }
    
    pub async fn download_file(
        &self,
        filename: &str,
        destination: &Path,
    ) -> Result<PathBuf> {
        let mut mirrors = self.config.patcher.mirrors.clone();
        mirrors.sort_by_key(|m| m.priority);
        
        let mut last_error = None;
        
        for mirror in &mirrors {
            if mirror.url.is_empty() {
                warn!("Skipping mirror {} with empty URL", mirror.name);
                continue;
            }
            
            let url = format!("{}/{}", mirror.url, filename);
            info!("Attempting download from mirror: {} ({})", mirror.name, url);
            
            match self.download_from_url(&url, destination).await {
                Ok(path) => {
                    info!("Successfully downloaded from mirror: {}", mirror.name);
                    return Ok(path);
                }
                Err(e) => {
                    warn!("Failed to download from mirror {}: {}", mirror.name, e);
                    last_error = Some(e);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| {
            Error::DownloadFailed("All mirrors failed".to_string())
        }))
    }
    
    async fn download_from_url(
        &self,
        url: &str,
        destination: &Path,
    ) -> Result<PathBuf> {
        debug!("Downloading: {}", url);
        
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "HTTP error: {}",
                response.status()
            )));
        }
        
        let _total_size = response.content_length().unwrap_or(0);
        let mut _downloaded: u64 = 0;
        
        let filepath = destination.to_path_buf();
        tokio::fs::create_dir_all(filepath.parent().unwrap()).await?;
        let mut file = File::create(&filepath).await?;
        
        let mut stream = response.bytes_stream();
        
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk).await?;
            
            _downloaded += chunk.len() as u64;
        }
        
        file.flush().await?;
        
        info!("Download completed: {:?}", filepath);
        Ok(filepath)
    }
    
    pub async fn download_patch_list(&self) -> Result<Vec<PatchInfo>> {
        let url = &self.config.patcher.patch_list_url;
        info!("Downloading patch list from: {}", url);
        
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "Failed to download patch list: HTTP {}",
                response.status()
            )));
        }
        
        let content = response.text().await?;
        let patches = self.parse_patch_list(&content)?;
        
        info!("Found {} patches", patches.len());
        Ok(patches)
    }
    
    fn parse_patch_list(&self, content: &str) -> Result<Vec<PatchInfo>> {
        let mut patches = Vec::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            let filename = parts[0].to_string();
            let checksum = if parts.len() > 1 {
                Some(parts[1].to_string())
            } else {
                None
            };
            
            patches.push(PatchInfo { filename, checksum });
        }
        
        Ok(patches)
    }
    
    pub async fn verify_checksum(&self, file_path: &Path, expected: &str) -> Result<bool> {
        if !self.config.patcher.verify_checksums {
            return Ok(true);
        }
        
        let data = tokio::fs::read(file_path).await?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let result = hasher.finalize();
        let hash = format!("{:x}", result);
        
        Ok(hash == expected)
    }
}

#[derive(Debug, Clone)]
pub struct PatchInfo {
    pub filename: String,
    pub checksum: Option<String>,
}

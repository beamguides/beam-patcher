use crate::{Config, Error, Result};
use futures::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tracing::{debug, info, warn};
use std::collections::HashSet;

pub struct Downloader {
    client: Client,
    config: Config,
    cache_file: PathBuf,
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
        
        let cache_file = if let Some(game_dir) = &config.app.game_directory {
            Path::new(game_dir).join(".patch_cache").join("applied_patches.txt")
        } else {
            let exe_dir = crate::get_executable_dir()?;
            exe_dir.join(".patch_cache").join("applied_patches.txt")
        };
        
        if let Some(parent) = cache_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok(Downloader { client, config, cache_file })
    }
    
    fn load_applied_patches(&self) -> Result<HashSet<String>> {
        let mut applied = HashSet::new();
        
        if self.cache_file.exists() {
            let content = std::fs::read_to_string(&self.cache_file)?;
            for line in content.lines() {
                let line = line.trim();
                if !line.is_empty() {
                    applied.insert(line.to_string());
                }
            }
        }
        
        Ok(applied)
    }
    
    pub fn mark_patch_applied(&self, filename: &str) -> Result<()> {
        use std::io::Write;
        // O(1) append + fsync — avoids rewriting whole cache every call.
        // Duplicates are tolerated; load_applied_patches dedupes via HashSet.
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.cache_file)?;
        writeln!(file, "{}", filename)?;
        file.sync_data()?;
        info!("Marked patch as applied: {}", filename);
        Ok(())
    }
    
    pub fn clear_cache(&self) -> Result<()> {
        if self.cache_file.exists() {
            std::fs::remove_file(&self.cache_file)?;
        }
        Ok(())
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

        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Atomic write: stream into a temp file, then rename. Interrupted
        // downloads leave behind a .part file rather than a corrupt destination.
        let tmp_path = tmp_path_for(destination);
        {
            let file = File::create(&tmp_path).await?;
            let mut writer = BufWriter::with_capacity(64 * 1024, file);

            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                writer.write_all(&chunk).await?;
            }
            writer.flush().await?;
        }
        tokio::fs::rename(&tmp_path, destination).await?;

        info!("Download completed: {:?}", destination);
        Ok(destination.to_path_buf())
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
        let all_patches = self.parse_patch_list(&content)?;
        
        let applied_patches = self.load_applied_patches()?;
        
        let pending_patches: Vec<PatchInfo> = all_patches
            .into_iter()
            .filter(|patch| !applied_patches.contains(&patch.filename))
            .collect();
        
        info!("Found {} total patches, {} already applied, {} pending", 
              pending_patches.len() + applied_patches.len(),
              applied_patches.len(),
              pending_patches.len());
        
        Ok(pending_patches)
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
            let size = if parts.len() > 2 {
                parts[2].parse::<u64>().ok()
            } else {
                None
            };
            
            patches.push(PatchInfo { filename, checksum, size });
        }
        
        Ok(patches)
    }
    
    pub async fn verify_checksum(&self, file_path: &Path, expected: &str) -> Result<bool> {
        if !self.config.patcher.verify_checksums {
            return Ok(true);
        }

        // Streaming hash — avoids loading the entire patch (potentially GBs) into RAM.
        let mut file = File::open(file_path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 { break; }
            hasher.update(&buffer[..n]);
        }
        let hash = format!("{:x}", hasher.finalize());

        Ok(hash.eq_ignore_ascii_case(expected))
    }
    
    pub async fn download_file_with_progress<F>(
        &self,
        filename: &str,
        destination: &Path,
        mut progress_callback: F,
    ) -> Result<PathBuf>
    where
        F: FnMut(u64, u64) + Send + 'static,
    {
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
            
            match self.download_from_url_with_progress(&url, destination, &mut progress_callback).await {
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
    
    async fn download_from_url_with_progress<F>(
        &self,
        url: &str,
        destination: &Path,
        progress_callback: &mut F,
    ) -> Result<PathBuf>
    where
        F: FnMut(u64, u64),
    {
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;

        if let Some(parent) = destination.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let tmp_path = tmp_path_for(destination);
        {
            let file = File::create(&tmp_path).await?;
            let mut writer = BufWriter::with_capacity(64 * 1024, file);

            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                writer.write_all(&chunk).await?;
                downloaded += chunk.len() as u64;
                progress_callback(downloaded, total_size);
            }
            writer.flush().await?;
        }
        tokio::fs::rename(&tmp_path, destination).await?;

        Ok(destination.to_path_buf())
    }
}

fn tmp_path_for(destination: &Path) -> PathBuf {
    let mut s = destination.as_os_str().to_owned();
    s.push(".part");
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tmp_path_appends_part_extension() {
        let p = tmp_path_for(Path::new("C:\\game\\data.grf"));
        assert_eq!(p.to_string_lossy(), "C:\\game\\data.grf.part");
    }

    #[test]
    fn tmp_path_handles_files_with_dots() {
        let p = tmp_path_for(Path::new("/tmp/patch.0001.thor"));
        assert_eq!(p.to_string_lossy(), "/tmp/patch.0001.thor.part");
    }

    #[test]
    fn tmp_path_handles_no_extension() {
        let p = tmp_path_for(Path::new("/tmp/noext"));
        assert_eq!(p.to_string_lossy(), "/tmp/noext.part");
    }
}

#[derive(Debug, Clone)]
pub struct PatchInfo {
    pub filename: String,
    pub checksum: Option<String>,
    pub size: Option<u64>,
}

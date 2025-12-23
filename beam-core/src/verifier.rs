use crate::{Config, Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileManifest {
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub checksum: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub total_files: usize,
    pub verified: usize,
    pub corrupted: Vec<String>,
    pub missing: Vec<String>,
}

pub struct Verifier {
    #[allow(dead_code)]
    config: Config,
    manifest_url: String,
}

impl Verifier {
    pub fn new(config: Config, manifest_url: String) -> Result<Self> {
        Ok(Verifier {
            config,
            manifest_url,
        })
    }
    
    pub async fn verify_game_files(&self) -> Result<VerificationResult> {
        info!("Starting game file verification");
        
        let manifest = self.download_manifest().await?;
        let mut result = VerificationResult {
            total_files: manifest.files.len(),
            verified: 0,
            corrupted: Vec::new(),
            missing: Vec::new(),
        };
        
        for file_entry in &manifest.files {
            let file_path = PathBuf::from(&file_entry.path);
            
            if !file_path.exists() {
                warn!("Missing file: {}", file_entry.path);
                result.missing.push(file_entry.path.clone());
                continue;
            }
            
            match self.verify_file(&file_path, &file_entry.checksum).await {
                Ok(true) => {
                    debug!("File verified: {}", file_entry.path);
                    result.verified += 1;
                }
                Ok(false) => {
                    warn!("Corrupted file: {}", file_entry.path);
                    result.corrupted.push(file_entry.path.clone());
                }
                Err(e) => {
                    warn!("Failed to verify {}: {}", file_entry.path, e);
                    result.corrupted.push(file_entry.path.clone());
                }
            }
        }
        
        info!(
            "Verification complete: {}/{} verified, {} corrupted, {} missing",
            result.verified,
            result.total_files,
            result.corrupted.len(),
            result.missing.len()
        );
        
        Ok(result)
    }
    
    async fn download_manifest(&self) -> Result<FileManifest> {
        info!("Downloading file manifest from: {}", self.manifest_url);
        
        let client = reqwest::Client::new();
        let response = client.get(&self.manifest_url).send().await?;
        
        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "Failed to download manifest: HTTP {}",
                response.status()
            )));
        }
        
        let manifest: FileManifest = response.json().await?;
        info!("Manifest downloaded: {} files", manifest.files.len());
        
        Ok(manifest)
    }
    
    async fn verify_file(&self, file_path: &Path, expected_checksum: &str) -> Result<bool> {
        let mut file = File::open(file_path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        let result = hasher.finalize();
        let hash = format!("{:x}", result);
        
        Ok(hash == expected_checksum)
    }
    
    pub async fn compute_file_checksum(file_path: &Path) -> Result<String> {
        let mut file = File::open(file_path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }
}

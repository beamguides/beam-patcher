use crate::{Config, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatusResult {
    pub client_exe_status: String,
    pub server_status: String,
    pub files_checked: usize,
    pub corrupted_files: usize,
}

pub struct ClientChecker {
    config: Config,
}

impl ClientChecker {
    pub fn new(config: Config) -> Self {
        ClientChecker { config }
    }
    
    pub async fn check_client_integrity(&self) -> Result<ClientStatusResult> {
        let game_dir = self.config.app.game_directory.as_ref()
            .ok_or_else(|| crate::Error::InvalidConfig("Game directory not set".to_string()))?;
        
        let client_exe = Path::new(game_dir).join(&self.config.app.client_exe);
        
        let kro_client_status = if client_exe.exists() {
            "OK".to_string()
        } else {
            "Missing".to_string()
        };
        
        let (files_checked, corrupted_files) = self.verify_critical_files(game_dir).await?;
        
        let server_status = if corrupted_files == 0 && client_exe.exists() {
            "OK".to_string()
        } else if !client_exe.exists() {
            "Client Missing".to_string()
        } else {
            format!("{} files corrupted", corrupted_files)
        };
        
        Ok(ClientStatusResult {
            client_exe_status: kro_client_status,
            server_status,
            files_checked,
            corrupted_files,
        })
    }
    
    async fn verify_critical_files(&self, game_dir: &str) -> Result<(usize, usize)> {
        let critical_files = vec![
            self.config.app.client_exe.as_str(),
        ];
        
        let mut files_checked = 0;
        let mut corrupted_files = 0;
        
        for file_name in critical_files {
            let file_path = Path::new(game_dir).join(file_name);
            
            if file_path.exists() {
                files_checked += 1;
                debug!("Verified file exists: {}", file_name);
            } else {
                warn!("Missing critical file: {}", file_name);
                corrupted_files += 1;
            }
        }
        
        Ok((files_checked, corrupted_files))
    }
    
    #[allow(dead_code)]
    fn calculate_file_hash(&self, path: &Path) -> Result<String> {
        let content = fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }
}

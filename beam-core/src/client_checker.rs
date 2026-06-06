use crate::{Config, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientStatusResult {
    pub client_exe_status: String,
    pub server_status: String,
    pub files_checked: usize,
    pub corrupted_files: usize,
    /// SHA-256 of the client executable, lowercase hex. `None` when the file is
    /// missing or unreadable.
    #[serde(default)]
    pub client_exe_sha256: Option<String>,
    /// Total size in bytes of the client executable, when present.
    #[serde(default)]
    pub client_exe_size: Option<u64>,
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
        let game_dir = PathBuf::from(game_dir);

        let client_exe = game_dir.join(&self.config.app.client_exe);

        // Status flags + integrity sniffing for the client executable.
        let (client_exe_status, client_exe_sha256, client_exe_size, client_ok) =
            inspect_client_exe(&client_exe).await;

        // Critical-file presence check (kept lightweight — full manifest
        // verification lives in `Verifier`).
        let (files_checked, corrupted_files) = self.verify_critical_files(&game_dir).await?;

        let server_status = if !client_ok {
            client_exe_status.clone()
        } else if corrupted_files == 0 {
            "OK".to_string()
        } else {
            format!("{} files corrupted", corrupted_files)
        };

        Ok(ClientStatusResult {
            client_exe_status,
            server_status,
            files_checked,
            corrupted_files,
            client_exe_sha256,
            client_exe_size,
        })
    }

    async fn verify_critical_files(&self, game_dir: &Path) -> Result<(usize, usize)> {
        // Files that, if missing, mean the install is broken beyond patching.
        // The client_exe is intentionally NOT included here — it's reported via
        // client_exe_status with richer detail.
        let mut critical: Vec<String> = Vec::new();
        if let Some(setup) = &self.config.app.setup_exe {
            critical.push(setup.clone());
        }
        // Common Ragnarok runtime payloads that the client always needs.
        critical.extend([
            "data.grf".to_string(),
            "rdata.grf".to_string(),
        ]);

        let target_grf = &self.config.patcher.target_grf;
        if !critical.iter().any(|f| f.eq_ignore_ascii_case(target_grf)) {
            critical.push(target_grf.clone());
        }

        let mut files_checked = 0usize;
        let mut corrupted_files = 0usize;

        for file_name in &critical {
            let file_path = game_dir.join(file_name);
            files_checked += 1;
            match tokio::fs::metadata(&file_path).await {
                Ok(md) if md.len() > 0 => {
                    debug!("Critical file OK ({} bytes): {}", md.len(), file_name);
                }
                Ok(_) => {
                    warn!("Critical file is empty: {}", file_name);
                    corrupted_files += 1;
                }
                Err(_) => {
                    warn!("Missing critical file: {}", file_name);
                    corrupted_files += 1;
                }
            }
        }

        Ok((files_checked, corrupted_files))
    }
}

/// Inspect the client executable and surface a coarse status string plus
/// an integrity fingerprint (SHA-256) the UI can compare against an
/// expected value if it has one.
async fn inspect_client_exe(path: &Path) -> (String, Option<String>, Option<u64>, bool) {
    let metadata = match tokio::fs::metadata(path).await {
        Ok(m) => m,
        Err(_) => return ("Missing".to_string(), None, None, false),
    };

    let size = metadata.len();
    if size == 0 {
        return ("Empty (0 bytes)".to_string(), None, Some(0), false);
    }

    // A normal Ragnarok client is tens of MB; anything tiny is almost
    // certainly a stub left behind by a failed install.
    if size < 1024 * 1024 {
        return (
            format!("Suspicious size ({} bytes) — possible stub", size),
            None,
            Some(size),
            false,
        );
    }

    match sha256_file(path).await {
        Ok(hash) => ("OK".to_string(), Some(hash), Some(size), true),
        Err(e) => {
            warn!("Failed to hash {:?}: {}", path, e);
            (format!("Unreadable: {}", e), None, Some(size), false)
        }
    }
}

async fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

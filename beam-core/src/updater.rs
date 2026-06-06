use crate::{Config, Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version: String,
    pub download_url: String,
    pub changelog: String,
    pub required: bool,
}

pub struct Updater {
    config: Config,
    client: Client,
}

impl Updater {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Beam-Patcher/1.0")
            .build()?;
        
        Ok(Updater { config, client })
    }
    
    pub async fn check_for_updates(&self) -> Result<Option<VersionInfo>> {
        let updater_config = self.config.updater.as_ref()
            .ok_or_else(|| Error::InvalidConfig("Updater not configured".to_string()))?;
        
        if !updater_config.enabled {
            return Ok(None);
        }
        
        info!("Checking for updates at: {}", updater_config.check_url);
        
        let response = self.client.get(&updater_config.check_url).send().await?;
        
        if !response.status().is_success() {
            return Err(Error::UpdateFailed(format!(
                "Failed to check for updates: HTTP {}",
                response.status()
            )));
        }
        
        let version_info: VersionInfo = response.json().await?;
        
        if version_info.version != self.config.app.version {
            info!("Update available: {} -> {}", self.config.app.version, version_info.version);
            Ok(Some(version_info))
        } else {
            info!("Already up to date");
            Ok(None)
        }
    }
    
    pub async fn perform_update(&self, version_info: &VersionInfo) -> Result<()> {
        info!("Downloading update from: {}", version_info.download_url);

        // Defaults track the upstream repo. Operators can override per-build
        // via UpdaterConfig.github_repo without recompiling.
        let (owner, name, bin_name) = match self
            .config
            .updater
            .as_ref()
            .and_then(|u| u.github_repo.as_ref())
        {
            Some(r) => (r.owner.clone(), r.name.clone(), r.bin_name.clone()),
            None => (
                "beamguides".to_string(),
                "beam-patcher".to_string(),
                "beam-patcher".to_string(),
            ),
        };

        // `self_update` is blocking — keep it off the async runtime.
        let current_version = self.config.app.version.clone();
        let status = tokio::task::spawn_blocking(move || {
            let update_builder = self_update::backends::github::Update::configure()
                .repo_owner(&owner)
                .repo_name(&name)
                .bin_name(&bin_name)
                .current_version(&current_version)
                .build()
                .map_err(|e| e.to_string())?;
            update_builder.update().map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| Error::UpdateFailed(format!("Updater task join error: {}", e)))?
        .map_err(Error::UpdateFailed)?;

        info!("Update completed: {:?}", status);
        Ok(())
    }
}

use crate::{Config, Downloader, Error, Result};
use beam_formats::{grf::Grf, gpf::Gpf, rgz::Rgz, thor::Thor, beam::BeamArchive};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

pub struct Patcher {
    config: Config,
    downloader: Downloader,
    temp_dir: PathBuf,
}

impl Patcher {
    pub fn new(config: Config) -> Result<Self> {
        let downloader = Downloader::new(config.clone())?;
        let temp_dir = std::env::temp_dir().join("beam_patcher");
        std::fs::create_dir_all(&temp_dir)?;
        
        Ok(Patcher {
            config,
            downloader,
            temp_dir,
        })
    }
    
    pub async fn run_full_patch(&self) -> Result<()> {
        info!("Starting full patch process");
        
        let patches = self.downloader.download_patch_list().await?;
        info!("Found {} patches to apply", patches.len());
        
        for (idx, patch) in patches.iter().enumerate() {
            info!("Processing patch {}/{}: {}", idx + 1, patches.len(), patch.filename);
            
            let patch_path = self.temp_dir.join(&patch.filename);
            
            self.downloader
                .download_file(&patch.filename, &patch_path)
                .await?;
            
            if let Some(checksum) = &patch.checksum {
                if !self.downloader.verify_checksum(&patch_path, checksum).await? {
                    return Err(Error::PatchFailed(format!(
                        "Checksum mismatch for {}",
                        patch.filename
                    )));
                }
            }
            
            self.apply_patch(&patch_path).await?;
            
            tokio::fs::remove_file(&patch_path).await?;
        }
        
        info!("All patches applied successfully");
        Ok(())
    }
    
    pub async fn apply_patch(&self, patch_path: &Path) -> Result<()> {
        let extension = patch_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        info!("Applying patch: {:?} (type: {})", patch_path, extension);
        
        match extension.to_lowercase().as_str() {
            "beam" => self.apply_beam_patch(patch_path).await,
            "thor" => self.apply_thor_patch(patch_path).await,
            "rgz" => self.apply_rgz_patch(patch_path).await,
            "gpf" => self.apply_gpf_patch(patch_path).await,
            _ => {
                warn!("Unknown patch format: {}", extension);
                Err(Error::PatchFailed(format!("Unknown patch format: {}", extension)))
            }
        }
    }
    
    async fn apply_beam_patch(&self, patch_path: &Path) -> Result<()> {
        info!("Applying BEAM patch with MD5 verification");
        let beam = BeamArchive::open(patch_path)?;
        
        let grf_path = self.get_grf_path()?;
        let mut grf = self.open_or_create_grf(&grf_path)?;
        
        for filename in beam.list_files() {
            info!("Extracting and verifying: {}", filename);
            
            if !beam.verify_file(filename)? {
                return Err(Error::PatchFailed(format!(
                    "MD5 verification failed for: {}",
                    filename
                )));
            }
            
            let data = beam.extract_file(filename)?;
            info!("Patching file: {} ({} bytes)", filename, data.len());
            grf.patch_file(filename, &data)?;
        }
        
        info!("Saving GRF file table...");
        grf.save()?;
        
        info!("BEAM patch applied successfully with all checksums verified");
        Ok(())
    }
    
    async fn apply_thor_patch(&self, patch_path: &Path) -> Result<()> {
        let thor = Thor::open(patch_path)?;
        
        let grf_path = self.get_grf_path()?;
        let mut grf = self.open_or_create_grf(&grf_path)?;
        
        for entry in thor.get_entries() {
            match entry {
                beam_formats::thor::ThorEntry::Add { filename, data } => {
                    info!("Adding/updating file: {}", filename);
                    grf.patch_file(filename, data)?;
                }
                beam_formats::thor::ThorEntry::Remove { filename } => {
                    info!("Removing file: {}", filename);
                }
            }
        }
        
        info!("Saving GRF file table...");
        grf.save()?;
        
        Ok(())
    }
    
    async fn apply_rgz_patch(&self, patch_path: &Path) -> Result<()> {
        let rgz = Rgz::open(patch_path)?;
        
        let grf_path = self.get_grf_path()?;
        let mut grf = self.open_or_create_grf(&grf_path)?;
        
        for entry in rgz.get_entries() {
            match entry {
                beam_formats::rgz::RgzEntry::File { name, data } => {
                    info!("Adding file: {}", name);
                    grf.patch_file(name, data)?;
                }
                beam_formats::rgz::RgzEntry::Directory { name } => {
                    debug!("Creating directory: {}", name);
                }
            }
        }
        
        info!("Saving GRF file table...");
        grf.save()?;
        
        Ok(())
    }
    
    async fn apply_gpf_patch(&self, patch_path: &Path) -> Result<()> {
        let gpf = Gpf::open(patch_path)?;
        
        let grf_path = self.get_grf_path()?;
        let mut grf = self.open_or_create_grf(&grf_path)?;
        
        for filename in gpf.list_files() {
            info!("Patching file: {}", filename);
            let data = gpf.extract_file(filename)?;
            grf.patch_file(filename, &data)?;
        }
        
        info!("Saving GRF file table...");
        grf.save()?;
        
        Ok(())
    }
    
    pub async fn manual_patch(&self, patch_path: &Path) -> Result<()> {
        if !self.config.patcher.allow_manual_patch {
            return Err(Error::PatchFailed(
                "Manual patching is disabled".to_string()
            ));
        }
        
        info!("Applying manual patch: {:?}", patch_path);
        self.apply_patch(patch_path).await
    }
    
    fn get_grf_path(&self) -> Result<PathBuf> {
        let grf_filename = &self.config.patcher.target_grf;
        
        if let Some(game_dir) = &self.config.app.game_directory {
            let path = Path::new(game_dir).join(grf_filename);
            Ok(path)
        } else {
            Ok(PathBuf::from(grf_filename))
        }
    }
    
    fn open_or_create_grf(&self, path: &Path) -> Result<Grf> {
        if path.exists() {
            info!("Opening existing GRF: {:?}", path);
            Ok(Grf::open(path)?)
        } else {
            info!("GRF not found, creating new: {:?}", path);
            Ok(Grf::create_new(path)?)
        }
    }
}

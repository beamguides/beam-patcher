use crate::{Config, Downloader, Error, Result};
use crate::downloader::PatchInfo;
use beam_formats::{grf::Grf, gpf::Gpf, rgz::Rgz, thor::Thor, beam::BeamArchive};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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

    pub async fn check_available_patches(&self) -> Result<usize> {
        info!("Checking for available patches");

        let patches = self.downloader.download_patch_list().await?;
        let patch_count = patches.len();

        if patch_count > 0 {
            info!("Found {} patches available for download", patch_count);
        } else {
            info!("No patches available, client is up to date");
        }

        Ok(patch_count)
    }

    pub async fn get_patch_list(&self) -> Result<Vec<PatchInfo>> {
        self.downloader.download_patch_list().await
    }

    /// Apply a single patch with download progress, opening + saving the GRF
    /// for just this patch. Prefer `download_and_apply_patches` when you have
    /// more than one patch to apply — the batched form opens the GRF once and
    /// rebuilds it once, which is dramatically faster on large GRF files.
    pub async fn download_and_apply_patch<F>(
        &self,
        patch: &PatchInfo,
        progress_callback: F,
    ) -> Result<()>
    where
        F: FnMut(u64, u64) + Send + 'static,
    {
        let patch_path = self.temp_dir.join(&patch.filename);

        self.downloader
            .download_file_with_progress(&patch.filename, &patch_path, progress_callback)
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

        self.downloader.mark_patch_applied(&patch.filename)?;

        tokio::fs::remove_file(&patch_path).await?;

        Ok(())
    }

    /// Batched version of the patch flow:
    /// downloads each patch, queues changes into a single in-memory GRF,
    /// and saves the GRF exactly once at the end. Saves O(N) full-GRF rewrites
    /// down to one. `progress` is called with `(patch_index, patch_total, downloaded, total_bytes)`.
    pub async fn download_and_apply_patches<F>(
        &self,
        patches: &[PatchInfo],
        progress: F,
    ) -> Result<()>
    where
        F: FnMut(usize, usize, u64, u64) + Send + 'static,
    {
        if patches.is_empty() {
            return Ok(());
        }

        let grf_path = self.get_grf_path()?;
        let mut grf = self.open_or_create_grf(&grf_path)?;
        let total = patches.len();

        // Wrap the user-supplied progress callback so each download can take its
        // own per-chunk closure that's `'static + Send` (which the downloader API
        // requires). Real per-chunk progress is restored this way.
        let progress = Arc::new(Mutex::new(progress));

        // Don't write to applied_patches.txt yet — defer until grf.save() succeeds
        // so a save failure can't leave the cache lying about what's on disk.
        let mut staged: Vec<String> = Vec::with_capacity(patches.len());

        for (idx, patch) in patches.iter().enumerate() {
            info!("Processing patch {}/{}: {}", idx + 1, total, patch.filename);
            let patch_path = self.temp_dir.join(&patch.filename);

            let progress_for_dl = Arc::clone(&progress);
            self.downloader
                .download_file_with_progress(
                    &patch.filename,
                    &patch_path,
                    move |downloaded, t| {
                        if let Ok(mut cb) = progress_for_dl.lock() {
                            (*cb)(idx, total, downloaded, t);
                        }
                    },
                )
                .await?;

            if let Some(checksum) = &patch.checksum {
                if !self.downloader.verify_checksum(&patch_path, checksum).await? {
                    return Err(Error::PatchFailed(format!(
                        "Checksum mismatch for {}",
                        patch.filename
                    )));
                }
            }

            self.apply_patch_into_grf(&patch_path, &mut grf).await?;
            staged.push(patch.filename.clone());
            let _ = tokio::fs::remove_file(&patch_path).await;
        }

        info!("Flushing {} batched patches to GRF: {:?}", staged.len(), grf_path);
        grf.save()?;

        // GRF persisted — now it's safe to record what's applied.
        for fname in &staged {
            self.downloader.mark_patch_applied(fname)?;
        }

        Ok(())
    }

    pub async fn run_full_patch(&self) -> Result<()> {
        info!("Starting full patch process");

        let patches = self.downloader.download_patch_list().await?;
        info!("Found {} patches to apply", patches.len());

        self.download_and_apply_patches(&patches, |idx, total, _, _| {
            debug!("Patch {}/{}", idx + 1, total);
        }).await?;

        info!("All patches applied successfully");
        Ok(())
    }

    /// One-shot apply: opens its own GRF, applies, saves.
    pub async fn apply_patch(&self, patch_path: &Path) -> Result<()> {
        let grf_path = self.get_grf_path()?;
        let mut grf = self.open_or_create_grf(&grf_path)?;
        self.apply_patch_into_grf(patch_path, &mut grf).await?;
        grf.save()?;
        Ok(())
    }

    /// Internal: stage the patch contents into a caller-owned GRF without saving.
    async fn apply_patch_into_grf(&self, patch_path: &Path, grf: &mut Grf) -> Result<()> {
        let extension = patch_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        info!("Applying patch: {:?} (type: {})", patch_path, extension);

        match extension.as_str() {
            "beam" => self.apply_beam_into(patch_path, grf),
            "thor" => self.apply_thor_into(patch_path, grf),
            "rgz"  => self.apply_rgz_into(patch_path, grf),
            "gpf"  => self.apply_gpf_into(patch_path, grf),
            other => {
                warn!("Unknown patch format: {}", other);
                Err(Error::PatchFailed(format!("Unknown patch format: {}", other)))
            }
        }
    }

    fn apply_beam_into(&self, patch_path: &Path, grf: &mut Grf) -> Result<()> {
        info!("Applying BEAM patch with MD5 verification");
        let beam = BeamArchive::open(patch_path)?;

        for filename in beam.list_files() {
            info!("Extracting and verifying: {}", filename);

            if !beam.verify_file(filename)? {
                return Err(Error::PatchFailed(format!(
                    "MD5 verification failed for: {}",
                    filename
                )));
            }

            let data = beam.extract_file(filename)?;

            let entry = beam.get_entry(filename)
                .ok_or_else(|| Error::PatchFailed(format!("Entry not found: {}", filename)))?;

            let grf_filename = entry.grf_path.as_ref().unwrap_or(&entry.filename);

            info!("Patching file: {} -> {} ({} bytes)", filename, grf_filename, data.len());
            grf.patch_file(grf_filename, &data)?;
        }
        Ok(())
    }

    fn apply_thor_into(&self, patch_path: &Path, grf: &mut Grf) -> Result<()> {
        let thor = Thor::open(patch_path)?;

        for entry in thor.get_entries() {
            match entry {
                beam_formats::thor::ThorEntry::Add { filename, data } => {
                    info!("Adding/updating file: {}", filename);
                    grf.patch_file(filename, data)?;
                }
                beam_formats::thor::ThorEntry::Remove { filename } => {
                    info!("Removing file: {}", filename);
                    grf.remove_file(filename)?;
                }
            }
        }
        Ok(())
    }

    fn apply_rgz_into(&self, patch_path: &Path, grf: &mut Grf) -> Result<()> {
        let rgz = Rgz::open(patch_path)?;

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
        Ok(())
    }

    fn apply_gpf_into(&self, patch_path: &Path, grf: &mut Grf) -> Result<()> {
        let gpf = Gpf::open(patch_path)?;

        for filename in gpf.list_files() {
            info!("Patching file: {}", filename);
            let data = gpf.extract_file(filename)?;
            grf.patch_file(filename, &data)?;
        }
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
            Ok(Path::new(game_dir).join(grf_filename))
        } else {
            let exe_dir = crate::get_executable_dir()?;
            Ok(exe_dir.join(grf_filename))
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

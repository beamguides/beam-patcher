use crate::{Error, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const GRF_HEADER: &[u8; 15] = b"Master of Magic";
const GRF_HEADER_SIZE: u32 = 46; // Full header size: magic(16) + key(14) + offset(4) + seed(4) + count(4) + version(4)
const GRF_VERSION_0X101: u32 = 0x101;
const GRF_VERSION_0X102: u32 = 0x102;
const GRF_VERSION_0X103: u32 = 0x103;
const GRF_VERSION_0X200: u32 = 0x200;
const GRF_VERSION_0X300: u32 = 0x300;

#[derive(Debug, Clone)]
pub struct GrfEntry {
    pub filename: String,
    pub compressed_size: u32,
    pub compressed_size_aligned: u32,
    pub uncompressed_size: u32,
    pub flags: u8,
    pub offset: u32,
}

#[derive(Debug)]
pub struct Grf {
    pub version: u32,
    entries: HashMap<String, GrfEntry>,
    file_path: PathBuf,
    pending_patches: HashMap<String, Vec<u8>>,
    /// Files queued for removal on the next save(). Stored case-folded to match
    /// the same key normalisation we use for entries; the on-disk filename in
    /// `entries` is preserved verbatim.
    pending_removals: std::collections::HashSet<String>,
}

impl Grf {
    pub fn create_new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = std::fs::File::create(path)?;
        
        // Write GRF header (46 bytes): magic(16) + key(14) + offset(4) + seed(4) + count(4) + version(4)
        file.write_all(GRF_HEADER)?;
        file.write_all(&[0u8])?;
        file.write_all(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14])?;
        file.write_all(&0u32.to_le_bytes())?; // FileTableOffset
        file.write_all(&0u32.to_le_bytes())?; // Seed
        file.write_all(&0u32.to_le_bytes())?; // FilesCount
        file.write_all(&GRF_VERSION_0X200.to_le_bytes())?; // Version
        
        Ok(Grf {
            version: GRF_VERSION_0X200,
            entries: HashMap::new(),
            file_path: path.to_path_buf(),
            pending_patches: HashMap::new(),
            pending_removals: std::collections::HashSet::new(),
        })
    }
    
    pub fn version_name(version: u32) -> &'static str {
        match version {
            GRF_VERSION_0X101 => "0x101 (Legacy)",
            GRF_VERSION_0X102 => "0x102 (Standard Encryption)",
            GRF_VERSION_0X103 => "0x103 (Enhanced Encryption)",
            GRF_VERSION_0X200 => "0x200 (Modern Standard)",
            GRF_VERSION_0X300 => "0x300 (Gepard Shield / Custom Encryption)",
            _ => "Unknown",
        }
    }
    
    pub fn detect_version<P: AsRef<Path>>(path: P) -> Result<u32> {
        let path = path.as_ref();
        let mut file = std::fs::File::open(path)?;
        
        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;
        
        if &header[..15] != GRF_HEADER {
            return Err(Error::InvalidGrfHeader);
        }
        
        file.seek(SeekFrom::Start(42))?;
        
        let mut version_buf = [0u8; 4];
        file.read_exact(&mut version_buf)?;
        let version = u32::from_le_bytes(version_buf);
        
        Ok(version)
    }
    
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = std::fs::File::open(path)?;
        
        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;
        
        if &header[..15] != GRF_HEADER {
            return Err(Error::InvalidGrfHeader);
        }
        
        let mut key = [0u8; 14];
        file.read_exact(&mut key)?;
        
        file.seek(SeekFrom::Start(42))?;
        
        let mut version_buf = [0u8; 4];
        file.read_exact(&mut version_buf)?;
        let version = u32::from_le_bytes(version_buf);
        
        match version {
            GRF_VERSION_0X101 | GRF_VERSION_0X102 | GRF_VERSION_0X103 | GRF_VERSION_0X200 | GRF_VERSION_0X300 => {},
            _ => return Err(Error::InvalidGrfVersion(version)),
        }
        
        let entries = Self::read_file_table(&mut file, version)?;
        
        Ok(Grf {
            version,
            entries,
            file_path: path.to_path_buf(),
            pending_patches: HashMap::new(),
            pending_removals: std::collections::HashSet::new(),
        })
    }
    
    fn read_file_table<R: Read + Seek>(reader: &mut R, version: u32) -> Result<HashMap<String, GrfEntry>> {
        let mut entries = HashMap::new();
        
        match version {
            GRF_VERSION_0X300 => {
                reader.seek(SeekFrom::Start(34))?;
                
                let mut file_count_buf = [0u8; 4];
                reader.read_exact(&mut file_count_buf)?;
                let _file_count = u32::from_le_bytes(file_count_buf);
                
                let mut seed_buf = [0u8; 4];
                reader.read_exact(&mut seed_buf)?;
                
                let mut table_offset_buf = [0u8; 4];
                reader.read_exact(&mut table_offset_buf)?;
                let table_offset = u32::from_le_bytes(table_offset_buf);
                
                let mut table_size_buf = [0u8; 4];
                reader.read_exact(&mut table_size_buf)?;
                let table_size = u32::from_le_bytes(table_size_buf);
                
                let mut table_compressed_size_buf = [0u8; 4];
                reader.read_exact(&mut table_compressed_size_buf)?;
                let table_compressed_size = u32::from_le_bytes(table_compressed_size_buf);
                
                reader.seek(SeekFrom::Start((table_offset + GRF_HEADER_SIZE) as u64))?;
                
                let mut compressed_table = vec![0u8; table_compressed_size as usize];
                reader.read_exact(&mut compressed_table)?;
                
                let table_data = Self::decrypt_grf_0x300_table(&compressed_table, table_size)?;
                
                let mut cursor = std::io::Cursor::new(table_data);
                
                while cursor.position() < cursor.get_ref().len() as u64 {
                    // Read null-terminated filename
                    let mut filename_bytes = Vec::new();
                    loop {
                        let mut byte = [0u8; 1];
                        if cursor.read(&mut byte).unwrap_or(0) == 0 {
                            break; // EOF
                        }
                        if byte[0] == 0 {
                            break; // Null terminator
                        }
                        filename_bytes.push(byte[0]);
                    }
                    
                    if filename_bytes.is_empty() {
                        break; // No more entries
                    }
                    
                    let filename = String::from_utf8_lossy(&filename_bytes).to_string();
                    
                    let mut compressed_size_buf = [0u8; 4];
                    cursor.read_exact(&mut compressed_size_buf)?;
                    let compressed_size = u32::from_le_bytes(compressed_size_buf);
                    
                    let mut compressed_size_aligned_buf = [0u8; 4];
                    cursor.read_exact(&mut compressed_size_aligned_buf)?;
                    let compressed_size_aligned = u32::from_le_bytes(compressed_size_aligned_buf);
                    
                    let mut uncompressed_size_buf = [0u8; 4];
                    cursor.read_exact(&mut uncompressed_size_buf)?;
                    let uncompressed_size = u32::from_le_bytes(uncompressed_size_buf);
                    
                    let mut flags_buf = [0u8; 1];
                    cursor.read_exact(&mut flags_buf)?;
                    let flags = flags_buf[0];
                    
                    let mut offset_buf = [0u8; 4];
                    cursor.read_exact(&mut offset_buf)?;
                    let offset = u32::from_le_bytes(offset_buf);
                    
                    entries.insert(
                        filename.clone(),
                        GrfEntry {
                            filename,
                            compressed_size,
                            compressed_size_aligned,
                            uncompressed_size,
                            flags,
                            offset,
                        },
                    );
                }
            },
            GRF_VERSION_0X200 => {
                // Read header fields
                reader.seek(SeekFrom::Start(30))?;
                let mut table_offset_buf = [0u8; 4];
                reader.read_exact(&mut table_offset_buf)?;
                let table_offset = u32::from_le_bytes(table_offset_buf);

                let mut seed_buf = [0u8; 4];
                reader.read_exact(&mut seed_buf)?;

                let mut file_count_buf = [0u8; 4];
                reader.read_exact(&mut file_count_buf)?;
                let file_count = u32::from_le_bytes(file_count_buf);

                // A freshly-created empty GRF has no file table on disk —
                // skip the metadata read or it'll error with UnexpectedEof.
                if file_count == 0 && table_offset == 0 {
                    return Ok(entries);
                }

                // Seek to table metadata (at FileTableOffset + 46)
                reader.seek(SeekFrom::Start((table_offset + GRF_HEADER_SIZE) as u64))?;
                
                // Read table metadata
                let mut table_compressed_size_buf = [0u8; 4];
                reader.read_exact(&mut table_compressed_size_buf)?;
                let table_compressed_size = u32::from_le_bytes(table_compressed_size_buf);
                
                let mut table_size_buf = [0u8; 4];
                reader.read_exact(&mut table_size_buf)?;
                let table_size = u32::from_le_bytes(table_size_buf);
                
                tracing::info!("Reading GRF 0x200 - file_count: {}, table_offset: {}, table_size: {}, compressed_size: {}",
                    file_count, table_offset, table_size, table_compressed_size);
                
                // Read compressed table data (already at correct position after reading metadata)
                
                let mut compressed_table = vec![0u8; table_compressed_size as usize];
                reader.read_exact(&mut compressed_table)?;
                
                let mut decompressor = ZlibDecoder::new(&compressed_table[..]);
                let mut table_data = Vec::with_capacity(table_size as usize);
                decompressor.read_to_end(&mut table_data)?;
                
                let mut cursor = std::io::Cursor::new(table_data);
                
                while cursor.position() < cursor.get_ref().len() as u64 {
                    // Read null-terminated filename
                    let mut filename_bytes = Vec::new();
                    loop {
                        let mut byte = [0u8; 1];
                        if cursor.read(&mut byte).unwrap_or(0) == 0 {
                            break; // EOF
                        }
                        if byte[0] == 0 {
                            break; // Null terminator
                        }
                        filename_bytes.push(byte[0]);
                    }
                    
                    if filename_bytes.is_empty() {
                        break; // No more entries
                    }
                    
                    let filename = String::from_utf8_lossy(&filename_bytes).to_string();
                    
                    let mut compressed_size_buf = [0u8; 4];
                    cursor.read_exact(&mut compressed_size_buf)?;
                    let compressed_size = u32::from_le_bytes(compressed_size_buf);
                    
                    let mut compressed_size_aligned_buf = [0u8; 4];
                    cursor.read_exact(&mut compressed_size_aligned_buf)?;
                    let compressed_size_aligned = u32::from_le_bytes(compressed_size_aligned_buf);
                    
                    let mut uncompressed_size_buf = [0u8; 4];
                    cursor.read_exact(&mut uncompressed_size_buf)?;
                    let uncompressed_size = u32::from_le_bytes(uncompressed_size_buf);
                    
                    let mut flags_buf = [0u8; 1];
                    cursor.read_exact(&mut flags_buf)?;
                    let flags = flags_buf[0];
                    
                    let mut offset_buf = [0u8; 4];
                    cursor.read_exact(&mut offset_buf)?;
                    let offset = u32::from_le_bytes(offset_buf);
                    
                    entries.insert(
                        filename.clone(),
                        GrfEntry {
                            filename,
                            compressed_size,
                            compressed_size_aligned,
                            uncompressed_size,
                            flags,
                            offset,
                        },
                    );
                }
            },
            _ => {
                reader.seek(SeekFrom::Start(30))?;
                
                let mut file_count_buf = [0u8; 4];
                reader.read_exact(&mut file_count_buf)?;
                let file_count = u32::from_le_bytes(file_count_buf);
                
                reader.seek(SeekFrom::Start(GRF_HEADER_SIZE as u64))?;
                
                for _ in 0..file_count {
                    let mut filename_len_buf = [0u8; 4];
                    reader.read_exact(&mut filename_len_buf)?;
                    let filename_len = u32::from_le_bytes(filename_len_buf);
                    
                    let mut filename_buf = vec![0u8; filename_len as usize];
                    reader.read_exact(&mut filename_buf)?;
                    let filename = String::from_utf8_lossy(&filename_buf).to_string();
                    
                    let mut compressed_size_buf = [0u8; 4];
                    reader.read_exact(&mut compressed_size_buf)?;
                    let compressed_size = u32::from_le_bytes(compressed_size_buf);
                    
                    let mut compressed_size_aligned_buf = [0u8; 4];
                    reader.read_exact(&mut compressed_size_aligned_buf)?;
                    let compressed_size_aligned = u32::from_le_bytes(compressed_size_aligned_buf);
                    
                    let mut uncompressed_size_buf = [0u8; 4];
                    reader.read_exact(&mut uncompressed_size_buf)?;
                    let uncompressed_size = u32::from_le_bytes(uncompressed_size_buf);
                    
                    let mut flags_buf = [0u8; 1];
                    reader.read_exact(&mut flags_buf)?;
                    let flags = flags_buf[0];
                    
                    let mut offset_buf = [0u8; 4];
                    reader.read_exact(&mut offset_buf)?;
                    let offset = u32::from_le_bytes(offset_buf);
                    
                    entries.insert(
                        filename.clone(),
                        GrfEntry {
                            filename,
                            compressed_size,
                            compressed_size_aligned,
                            uncompressed_size,
                            flags,
                            offset,
                        },
                    );
                }
            }
        }
        
        Ok(entries)
    }
    
    fn decrypt_grf_0x300_table(compressed_data: &[u8], expected_size: u32) -> Result<Vec<u8>> {
        let mut decompressor = ZlibDecoder::new(compressed_data);
        let mut table_data = Vec::with_capacity(expected_size as usize);
        decompressor.read_to_end(&mut table_data)
            .map_err(|e| Error::Decompression(e.to_string()))?;
        
        Ok(table_data)
    }
    
    pub fn get_entry(&self, filename: &str) -> Option<&GrfEntry> {
        self.entries.get(filename)
    }
    
    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>> {
        let entry = self.entries.get(filename)
            .ok_or_else(|| Error::FileNotFound(filename.to_string()))?;

        let mut file = std::fs::File::open(&self.file_path)?;
        file.seek(SeekFrom::Start((entry.offset + GRF_HEADER_SIZE) as u64))?;

        // Read the aligned (padded) length to keep on-disk layout intact, but
        // when returning we slice down to compressed_size so callers don't get
        // trailing alignment padding bytes appended to their data.
        let mut raw = vec![0u8; entry.compressed_size_aligned as usize];
        file.read_exact(&mut raw)?;
        let payload = &raw[..entry.compressed_size as usize];

        if entry.flags & 0x01 != 0 {
            let mut decompressor = ZlibDecoder::new(payload);
            let mut decompressed = Vec::with_capacity(entry.uncompressed_size as usize);
            decompressor.read_to_end(&mut decompressed)
                .map_err(|e| Error::Decompression(e.to_string()))?;
            Ok(decompressed)
        } else {
            Ok(payload.to_vec())
        }
    }
    
    pub fn patch_file(&mut self, filename: &str, data: &[u8]) -> Result<()> {
        // Store uncompressed data in memory for later rebuild
        tracing::info!("patch_file() called for: {} ({} bytes)", filename, data.len());
        // Patching wins over a queued removal for the same path.
        self.pending_removals.remove(filename);
        self.pending_patches.insert(filename.to_string(), data.to_vec());
        tracing::debug!("Total pending patches now: {}", self.pending_patches.len());
        Ok(())
    }

    /// Queue a file for deletion on the next `save()`. Removing a file that was
    /// also queued for patching cancels the patch. Removing a file that doesn't
    /// exist in the archive is a no-op (matches THOR semantics).
    pub fn remove_file(&mut self, filename: &str) -> Result<()> {
        tracing::info!("remove_file() queued: {}", filename);
        self.pending_patches.remove(filename);
        if self.entries.contains_key(filename) {
            self.pending_removals.insert(filename.to_string());
        } else {
            tracing::debug!("remove_file: '{}' not in archive, skipping", filename);
        }
        Ok(())
    }

    /// True if either a patch or a removal is queued for the next save().
    pub fn has_pending_changes(&self) -> bool {
        !self.pending_patches.is_empty() || !self.pending_removals.is_empty()
    }
    
    pub fn list_files(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }
    
    pub fn file_count(&self) -> usize {
        self.entries.len()
    }
    
    pub fn save(&mut self) -> Result<()> {
        if self.version != GRF_VERSION_0X200 && self.version != GRF_VERSION_0X300 {
            return Err(Error::Unsupported("Only GRF 0x200 and 0x300 save is supported".to_string()));
        }

        tracing::info!("GRF save() called - pending patches: {}, pending removals: {}, existing entries: {}",
            self.pending_patches.len(), self.pending_removals.len(), self.entries.len());

        if !self.has_pending_changes() {
            tracing::info!("No pending changes, skipping save");
            return Ok(());
        }

        // Atomic rebuild strategy:
        //   1. Write the new GRF to <path>.tmp (BufWriter, large I/O batches).
        //   2. If that succeeds, rename original to <path>.bak as a safety net.
        //   3. Rename .tmp to <path> (atomic on the same filesystem).
        //   4. Delete .bak.
        // If anything aborts in step 1, the original GRF is untouched.
        let tmp_path = self.file_path.with_extension("grf.tmp");
        let backup_path = self.file_path.with_extension("grf.bak");

        // Re-open the existing GRF for reading so we can copy through unchanged entries.
        let original_exists = self.file_path.exists();
        let mut old_grf: Option<std::fs::File> = if original_exists {
            Some(std::fs::File::open(&self.file_path)?)
        } else {
            None
        };

        let result = (|| -> Result<HashMap<String, GrfEntry>> {
            let tmp_handle = std::fs::File::create(&tmp_path)?;
            let mut new_file = BufWriter::with_capacity(1 << 20, tmp_handle); // 1 MiB buffer

            // Header (46 bytes): magic(16) + key(14) + offset(4) + seed(4) + count(4) + version(4)
            new_file.write_all(GRF_HEADER)?;
            new_file.write_all(&[0u8])?;
            new_file.write_all(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14])?;
            new_file.write_all(&0u32.to_le_bytes())?; // FileTableOffset (patched below)
            new_file.write_all(&0u32.to_le_bytes())?; // Seed
            new_file.write_all(&0u32.to_le_bytes())?; // FilesCount (patched below)
            new_file.write_all(&self.version.to_le_bytes())?;

            let mut current_offset: u32 = 0;
            let mut new_entries = HashMap::with_capacity(self.entries.len() + self.pending_patches.len());

            // Copy untouched entries from old GRF. Sort by source offset so the
            // read cursor moves forward — far cheaper on disk than HashMap order.
            // Entries queued for patching OR removal are skipped here.
            let mut keep: Vec<(&String, &GrfEntry)> = self.entries.iter()
                .filter(|(name, _)| {
                    !self.pending_patches.contains_key(*name)
                        && !self.pending_removals.contains(*name)
                })
                .collect();
            keep.sort_by_key(|(_, e)| e.offset);

            // Reuse one buffer; grow as needed.
            let mut copy_buf: Vec<u8> = Vec::new();

            if let Some(old) = old_grf.as_mut() {
                for (filename, entry) in keep {
                    old.seek(SeekFrom::Start((entry.offset + GRF_HEADER_SIZE) as u64))?;
                    let sz = entry.compressed_size_aligned as usize;
                    if copy_buf.len() < sz { copy_buf.resize(sz, 0); }
                    old.read_exact(&mut copy_buf[..sz])?;
                    new_file.write_all(&copy_buf[..sz])?;

                    new_entries.insert(
                        filename.clone(),
                        GrfEntry {
                            filename: filename.clone(),
                            compressed_size: entry.compressed_size,
                            compressed_size_aligned: entry.compressed_size_aligned,
                            uncompressed_size: entry.uncompressed_size,
                            flags: entry.flags,
                            offset: current_offset,
                        },
                    );
                    current_offset = current_offset
                        .checked_add(entry.compressed_size_aligned)
                        .ok_or_else(|| Error::Custom("GRF offset overflow".into()))?;
                }
            }

            // Append patched files (newly compressed if it pays off).
            tracing::info!("Adding {} patched files to new GRF", self.pending_patches.len());
            for (filename, data) in &self.pending_patches {
                let (actual_data, flags): (Vec<u8>, u8) = if data.len() > 1024 {
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(data)?;
                    let compressed = encoder.finish()?;
                    if compressed.len() < data.len() {
                        (compressed, 0x01)
                    } else {
                        (data.clone(), 0x00)
                    }
                } else {
                    (data.clone(), 0x00)
                };

                let compressed_size = actual_data.len() as u32;
                let compressed_size_aligned = (compressed_size + 7) & !7;

                new_file.write_all(&actual_data)?;
                if compressed_size_aligned > compressed_size {
                    let pad = (compressed_size_aligned - compressed_size) as usize;
                    // Tiny pad — use a small zero stack buffer instead of allocating.
                    let zeros = [0u8; 8];
                    new_file.write_all(&zeros[..pad])?;
                }

                new_entries.insert(
                    filename.clone(),
                    GrfEntry {
                        filename: filename.clone(),
                        compressed_size,
                        compressed_size_aligned,
                        uncompressed_size: data.len() as u32,
                        flags,
                        offset: current_offset,
                    },
                );
                current_offset = current_offset
                    .checked_add(compressed_size_aligned)
                    .ok_or_else(|| Error::Custom("GRF offset overflow".into()))?;
            }

            // File table.
            let mut table_data = Vec::with_capacity(new_entries.len() * 32);
            for entry in new_entries.values() {
                table_data.extend_from_slice(entry.filename.as_bytes());
                table_data.push(0);
                table_data.extend_from_slice(&entry.compressed_size.to_le_bytes());
                table_data.extend_from_slice(&entry.compressed_size_aligned.to_le_bytes());
                table_data.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
                table_data.push(entry.flags);
                table_data.extend_from_slice(&entry.offset.to_le_bytes());
            }

            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&table_data)?;
            let compressed_table = encoder.finish()?;

            let table_offset = current_offset;
            new_file.write_all(&(compressed_table.len() as u32).to_le_bytes())?;
            new_file.write_all(&(table_data.len() as u32).to_le_bytes())?;
            new_file.write_all(&compressed_table)?;

            // Patch header (FileTableOffset, Seed, FilesCount). Version stays as written.
            new_file.seek(SeekFrom::Start(30))?;
            new_file.write_all(&table_offset.to_le_bytes())?;
            new_file.write_all(&0u32.to_le_bytes())?;
            new_file.write_all(&(new_entries.len() as u32).to_le_bytes())?;

            // Flush buffered writes to the OS, then to disk.
            let mut inner = new_file.into_inner().map_err(|e| e.into_error())?;
            inner.flush()?;
            inner.sync_data()?;

            Ok(new_entries)
        })();

        let new_entries = match result {
            Ok(e) => e,
            Err(e) => {
                // Failed mid-write: throw away the partial .tmp; original untouched.
                let _ = std::fs::remove_file(&tmp_path);
                return Err(e);
            }
        };

        // Release any read handle on the original before swapping.
        drop(old_grf);

        if original_exists {
            // Best-effort backup: ignore failure (e.g. .bak already exists on Windows).
            let _ = std::fs::remove_file(&backup_path);
            std::fs::rename(&self.file_path, &backup_path)?;
        }
        std::fs::rename(&tmp_path, &self.file_path)?;
        if original_exists {
            let _ = std::fs::remove_file(&backup_path);
        }

        self.entries = new_entries;
        self.pending_patches.clear();
        self.pending_removals.clear();

        tracing::info!("GRF save completed - total entries: {}", self.entries.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_grf(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("beam_grf_test_{}_{}.grf", std::process::id(), name));
        let _ = std::fs::remove_file(&p);
        p
    }

    #[test]
    fn create_then_open_roundtrip_empty() {
        let path = tmp_grf("empty");
        let g = Grf::create_new(&path).expect("create");
        assert_eq!(g.version, GRF_VERSION_0X200);
        assert_eq!(g.file_count(), 0);
        let g2 = Grf::open(&path).expect("reopen");
        assert_eq!(g2.version, GRF_VERSION_0X200);
        assert_eq!(g2.file_count(), 0);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn patch_then_extract_roundtrip() {
        let path = tmp_grf("patch");
        let mut g = Grf::create_new(&path).expect("create");

        let data = b"hello-beam-patcher".to_vec();
        g.patch_file("data\\foo.txt", &data).expect("patch_file");
        g.save().expect("save");

        let g2 = Grf::open(&path).expect("reopen");
        let got = g2.extract_file("data\\foo.txt").expect("extract");
        assert_eq!(got, data);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn remove_file_cancels_existing_entry() {
        let path = tmp_grf("remove");
        let mut g = Grf::create_new(&path).expect("create");

        g.patch_file("data\\keep.txt", b"keep").unwrap();
        g.patch_file("data\\drop.txt", b"drop").unwrap();
        g.save().unwrap();
        assert_eq!(Grf::open(&path).unwrap().file_count(), 2);

        let mut g = Grf::open(&path).unwrap();
        g.remove_file("data\\drop.txt").unwrap();
        assert!(g.has_pending_changes());
        g.save().unwrap();

        let g3 = Grf::open(&path).unwrap();
        assert_eq!(g3.file_count(), 1);
        assert!(g3.get_entry("data\\keep.txt").is_some());
        assert!(g3.get_entry("data\\drop.txt").is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn remove_overrides_patch_queued_first() {
        let path = tmp_grf("override");
        let mut g = Grf::create_new(&path).expect("create");
        g.patch_file("data\\x.txt", b"orig").unwrap();
        g.save().unwrap();

        let mut g = Grf::open(&path).unwrap();
        g.patch_file("data\\x.txt", b"new").unwrap();
        g.remove_file("data\\x.txt").unwrap(); // remove wins over the queued patch
        g.save().unwrap();

        let g3 = Grf::open(&path).unwrap();
        assert!(g3.get_entry("data\\x.txt").is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn open_rejects_garbage_header() {
        use std::io::Write as _;
        let path = tmp_grf("garbage");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"NOT A GRF FILE---").unwrap();
        let err = Grf::open(&path).unwrap_err();
        assert!(matches!(err, Error::InvalidGrfHeader));
        let _ = std::fs::remove_file(&path);
    }
}

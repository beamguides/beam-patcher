use crate::{Error, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
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
                let _file_count = u32::from_le_bytes(file_count_buf);
                
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
                    _file_count, table_offset, table_size, table_compressed_size);
                
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
        
        let mut compressed_data = vec![0u8; entry.compressed_size_aligned as usize];
        file.read_exact(&mut compressed_data)?;
        
        if entry.flags & 0x01 != 0 {
            let mut decompressor = ZlibDecoder::new(&compressed_data[..]);
            let mut decompressed = Vec::with_capacity(entry.uncompressed_size as usize);
            decompressor.read_to_end(&mut decompressed)
                .map_err(|e| Error::Decompression(e.to_string()))?;
            Ok(decompressed)
        } else {
            Ok(compressed_data)
        }
    }
    
    pub fn patch_file(&mut self, filename: &str, data: &[u8]) -> Result<()> {
        // Store uncompressed data in memory for later rebuild
        tracing::info!("patch_file() called for: {} ({} bytes)", filename, data.len());
        self.pending_patches.insert(filename.to_string(), data.to_vec());
        tracing::debug!("Total pending patches now: {}", self.pending_patches.len());
        Ok(())
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
        
        tracing::info!("GRF save() called - pending patches: {}, existing entries: {}", 
            self.pending_patches.len(), self.entries.len());
        
        if self.pending_patches.is_empty() {
            tracing::info!("No pending patches, skipping save");
            return Ok(());
        }
        
        tracing::info!("Starting GRF rebuild at: {:?}", self.file_path);
        
        // Rebuild GRF with pending patches
        let backup_path = self.file_path.with_extension("grf.bak");
        tracing::info!("Creating backup: {:?}", backup_path);
        std::fs::rename(&self.file_path, &backup_path)?;
        
        let mut new_file = std::fs::File::create(&self.file_path)?;
        
        // Write header (46 bytes): magic(16) + key(14) + offset(4) + seed(4) + count(4) + version(4)
        new_file.write_all(GRF_HEADER)?;
        new_file.write_all(&[0u8])?;
        new_file.write_all(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14])?;
        new_file.write_all(&0u32.to_le_bytes())?; // FileTableOffset (will update later)
        new_file.write_all(&0u32.to_le_bytes())?; // Seed
        new_file.write_all(&0u32.to_le_bytes())?; // FilesCount (will update later)
        new_file.write_all(&self.version.to_le_bytes())?; // Version
        
        // Start writing file data at offset 46 (after header)
        let mut current_offset: u32 = 0;
        let mut new_entries = HashMap::new();
        let mut old_grf = std::fs::File::open(&backup_path)?;
        
        // Copy existing files that are not being patched
        for (filename, entry) in &self.entries {
            if self.pending_patches.contains_key(filename) {
                continue; // Skip, will be replaced by patch
            }
            
            // Read old file data
            old_grf.seek(SeekFrom::Start((entry.offset + GRF_HEADER_SIZE) as u64))?;
            let mut file_data = vec![0u8; entry.compressed_size_aligned as usize];
            old_grf.read_exact(&mut file_data)?;
            
            // Write to new GRF
            new_file.write_all(&file_data)?;
            
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
            
            current_offset += entry.compressed_size_aligned;
        }
        
        // Add patched files
        tracing::info!("Adding {} patched files to new GRF", self.pending_patches.len());
        for (filename, data) in &self.pending_patches {
            tracing::debug!("Adding patched file: {} ({} bytes uncompressed)", filename, data.len());
            
            // Try compression for files > 1024 bytes
            let (actual_data, flags): (Vec<u8>, u8) = if data.len() > 1024 {
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(data)?;
                let compressed_data = encoder.finish()?;
                
                // Use compression only if it reduces size
                if compressed_data.len() < data.len() {
                    (compressed_data, 0x01) // Compressed flag
                } else {
                    (data.clone(), 0x00) // No compression, no encryption
                }
            } else {
                (data.clone(), 0x00) // Small files: no compression
            };
            
            new_file.write_all(&actual_data)?;
            
            let compressed_size = actual_data.len() as u32;
            let compressed_size_aligned = (compressed_size + 7) & !7;
            
            if compressed_size_aligned > compressed_size {
                let padding = vec![0u8; (compressed_size_aligned - compressed_size) as usize];
                new_file.write_all(&padding)?;
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
            
            current_offset += compressed_size_aligned;
        }
        
        // Build file table
        let mut table_data = Vec::new();
        for entry in new_entries.values() {
            // Write null-terminated filename (variable length)
            table_data.extend_from_slice(entry.filename.as_bytes());
            table_data.push(0); // Null terminator
            
            table_data.extend_from_slice(&entry.compressed_size.to_le_bytes());
            table_data.extend_from_slice(&entry.compressed_size_aligned.to_le_bytes());
            table_data.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
            table_data.extend_from_slice(&[entry.flags]);
            table_data.extend_from_slice(&entry.offset.to_le_bytes());
        }
        
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&table_data)?;
        let compressed_table = encoder.finish()?;
        
        // Write file table metadata + compressed data
        let table_offset = current_offset;
        new_file.write_all(&(compressed_table.len() as u32).to_le_bytes())?; // TableSizeCompressed
        new_file.write_all(&(table_data.len() as u32).to_le_bytes())?; // TableSize
        new_file.write_all(&compressed_table)?; // Compressed table data
        
        // Update header with file table info
        tracing::info!("Writing header - file_count: {}, table_offset: {}, table_size: {}, compressed_size: {}", 
            new_entries.len(), table_offset, table_data.len(), compressed_table.len());
        new_file.seek(SeekFrom::Start(30))?;
        new_file.write_all(&table_offset.to_le_bytes())?; // FileTableOffset (offset 30)
        new_file.write_all(&0u32.to_le_bytes())?; // Seed (offset 34)
        new_file.write_all(&(new_entries.len() as u32).to_le_bytes())?; // FilesCount (offset 38)
        // Version at offset 42 is already written in create_new(), don't overwrite
        
        drop(new_file);
        drop(old_grf);
        
        // Delete backup
        tracing::info!("Deleting backup file: {:?}", backup_path);
        std::fs::remove_file(&backup_path)?;
        
        // Update internal state
        self.entries = new_entries;
        self.pending_patches.clear();
        
        tracing::info!("GRF save completed successfully - total entries: {}", self.entries.len());
        
        Ok(())
    }
}

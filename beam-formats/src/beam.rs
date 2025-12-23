use crate::{Error, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const BEAM_MAGIC: &[u8; 4] = b"BEAM";
const BEAM_VERSION: u32 = 1;
const HEADER_SIZE: usize = 64;

#[derive(Debug, Clone)]
pub struct BeamEntry {
    pub filename: String,
    pub md5_hash: [u8; 16],
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub offset: u64,
}

#[derive(Debug)]
pub struct BeamArchive {
    pub version: u32,
    entries: HashMap<String, BeamEntry>,
    file_path: Option<PathBuf>,
    file_data: HashMap<String, Vec<u8>>,
}

impl BeamArchive {
    pub fn new() -> Self {
        BeamArchive {
            version: BEAM_VERSION,
            entries: HashMap::new(),
            file_path: None,
            file_data: HashMap::new(),
        }
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let mut file = std::fs::File::open(path)?;
        
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        
        if &magic != BEAM_MAGIC {
            return Err(Error::Custom("Invalid BEAM magic header".to_string()));
        }
        
        let mut version_buf = [0u8; 4];
        file.read_exact(&mut version_buf)?;
        let version = u32::from_le_bytes(version_buf);
        
        let mut entry_count_buf = [0u8; 4];
        file.read_exact(&mut entry_count_buf)?;
        let entry_count = u32::from_le_bytes(entry_count_buf);
        
        file.seek(SeekFrom::Start(HEADER_SIZE as u64))?;
        
        let mut entries = HashMap::new();
        
        for _ in 0..entry_count {
            let mut filename_len_buf = [0u8; 1];
            file.read_exact(&mut filename_len_buf)?;
            let filename_len = filename_len_buf[0] as usize;
            
            let mut filename_buf = vec![0u8; filename_len];
            file.read_exact(&mut filename_buf)?;
            let filename = String::from_utf8_lossy(&filename_buf).to_string();
            
            let mut md5_hash = [0u8; 16];
            file.read_exact(&mut md5_hash)?;
            
            let mut compressed_size_buf = [0u8; 4];
            file.read_exact(&mut compressed_size_buf)?;
            let compressed_size = u32::from_le_bytes(compressed_size_buf);
            
            let mut uncompressed_size_buf = [0u8; 4];
            file.read_exact(&mut uncompressed_size_buf)?;
            let uncompressed_size = u32::from_le_bytes(uncompressed_size_buf);
            
            let mut offset_buf = [0u8; 8];
            file.read_exact(&mut offset_buf)?;
            let offset = u64::from_le_bytes(offset_buf);
            
            entries.insert(
                filename.clone(),
                BeamEntry {
                    filename,
                    md5_hash,
                    compressed_size,
                    uncompressed_size,
                    offset,
                },
            );
        }
        
        Ok(BeamArchive {
            version,
            entries,
            file_path: Some(path.to_path_buf()),
            file_data: HashMap::new(),
        })
    }

    pub fn add_file(&mut self, filename: &str, data: &[u8]) -> Result<()> {
        let digest = md5::compute(data);
        let md5_hash: [u8; 16] = digest.0;
        
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data)?;
        let compressed_data = encoder.finish()?;
        
        self.file_data.insert(filename.to_string(), data.to_vec());
        
        self.entries.insert(
            filename.to_string(),
            BeamEntry {
                filename: filename.to_string(),
                md5_hash,
                compressed_size: compressed_data.len() as u32,
                uncompressed_size: data.len() as u32,
                offset: 0,
            },
        );
        
        Ok(())
    }

    pub fn add_file_from_path<P: AsRef<Path>>(&mut self, file_path: P, archive_path: &str) -> Result<()> {
        let data = std::fs::read(file_path)?;
        self.add_file(archive_path, &data)
    }

    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>> {
        let entry = self.entries.get(filename)
            .ok_or_else(|| Error::FileNotFound(filename.to_string()))?;
        
        let file_path = self.file_path.as_ref()
            .ok_or_else(|| Error::Custom("Archive not saved to file".to_string()))?;
        
        let mut file = std::fs::File::open(file_path)?;
        file.seek(SeekFrom::Start(entry.offset))?;
        
        let mut compressed_data = vec![0u8; entry.compressed_size as usize];
        file.read_exact(&mut compressed_data)?;
        
        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut decompressed = Vec::with_capacity(entry.uncompressed_size as usize);
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| Error::Decompression(e.to_string()))?;
        
        let digest = md5::compute(&decompressed);
        let calculated_hash: [u8; 16] = digest.0;
        
        if calculated_hash != entry.md5_hash {
            return Err(Error::Custom(format!(
                "MD5 checksum mismatch for file: {}",
                filename
            )));
        }
        
        Ok(decompressed)
    }

    pub fn save<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let mut file = std::fs::File::create(path)?;
        
        file.write_all(BEAM_MAGIC)?;
        file.write_all(&self.version.to_le_bytes())?;
        file.write_all(&(self.entries.len() as u32).to_le_bytes())?;
        file.write_all(&[0u8; 52])?; // Reserved
        
        let mut data_offset = HEADER_SIZE as u64;
        
        for entry in self.entries.values() {
            data_offset += 1; // filename_len
            data_offset += entry.filename.len() as u64;
            data_offset += 16; // md5
            data_offset += 4; // compressed_size
            data_offset += 4; // uncompressed_size
            data_offset += 8; // offset
        }
        
        let mut current_offset = data_offset;
        let mut compressed_files = Vec::new();
        
        for filename in self.entries.keys() {
            let data = if let Some(data) = self.file_data.get(filename) {
                data.clone()
            } else if self.file_path.is_some() {
                self.extract_file(filename)?
            } else {
                return Err(Error::Custom("No source data available".to_string()));
            };
            
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&data)?;
            let compressed = encoder.finish()?;
            
            compressed_files.push((filename.clone(), compressed));
        }
        
        for (filename, entry) in self.entries.iter_mut() {
            file.write_all(&[filename.len() as u8])?;
            file.write_all(filename.as_bytes())?;
            file.write_all(&entry.md5_hash)?;
            file.write_all(&entry.compressed_size.to_le_bytes())?;
            file.write_all(&entry.uncompressed_size.to_le_bytes())?;
            file.write_all(&current_offset.to_le_bytes())?;
            
            entry.offset = current_offset;
            current_offset += entry.compressed_size as u64;
        }
        
        for (_filename, compressed) in &compressed_files {
            file.write_all(compressed)?;
        }
        
        self.file_path = Some(path.to_path_buf());
        
        Ok(())
    }

    pub fn list_files(&self) -> Vec<&str> {
        self.entries.keys().map(|s| s.as_str()).collect()
    }

    pub fn get_entry(&self, filename: &str) -> Option<&BeamEntry> {
        self.entries.get(filename)
    }

    pub fn verify_file(&self, filename: &str) -> Result<bool> {
        let data = self.extract_file(filename)?;
        let entry = self.entries.get(filename).unwrap();
        
        let digest = md5::compute(&data);
        let calculated_hash: [u8; 16] = digest.0;
        
        Ok(calculated_hash == entry.md5_hash)
    }
}

impl Default for BeamArchive {
    fn default() -> Self {
        Self::new()
    }
}

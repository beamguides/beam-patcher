use crate::{Error, Result};
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use std::io::{Read, Write};
use std::path::Path;

const THOR_MAGIC: &[u8; 24] = b"ASSF (C) 2007 Aeomin DEV";

#[derive(Debug, Clone)]
pub enum ThorEntry {
    Add {
        filename: String,
        data: Vec<u8>,
    },
    Remove {
        filename: String,
    },
}

#[derive(Debug)]
pub struct Thor {
    pub entries: Vec<ThorEntry>,
}

impl Thor {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 24 {
            return Err(Error::InvalidThorHeader);
        }
        
        if &data[..24] != THOR_MAGIC {
            return Err(Error::InvalidThorHeader);
        }
        
        let mut cursor = std::io::Cursor::new(data);
        cursor.set_position(24);
        
        let mut use_grf_merging_buf = [0u8; 1];
        cursor.read_exact(&mut use_grf_merging_buf)?;
        
        let mut num_files_buf = [0u8; 4];
        cursor.read_exact(&mut num_files_buf)?;
        
        let mut mode_buf = [0u8; 2];
        cursor.read_exact(&mut mode_buf)?;
        let mode = i16::from_le_bytes(mode_buf);
        
        if mode != 0x30 && mode != 0x21 {
            return Err(Error::Custom(format!("Unsupported THOR mode: {:#x}", mode)));
        }
        
        let mut target_grf_len_buf = [0u8; 1];
        cursor.read_exact(&mut target_grf_len_buf)?;
        let target_grf_len = target_grf_len_buf[0] as usize;
        
        if target_grf_len > 0 {
            let mut target_grf_buf = vec![0u8; target_grf_len];
            cursor.read_exact(&mut target_grf_buf)?;
        }
        
        let mut file_table_compressed_len_buf = [0u8; 4];
        cursor.read_exact(&mut file_table_compressed_len_buf)?;
        let file_table_compressed_len = u32::from_le_bytes(file_table_compressed_len_buf) as usize;

        let mut file_table_offset_buf = [0u8; 4];
        cursor.read_exact(&mut file_table_offset_buf)?;
        let file_table_offset = u32::from_le_bytes(file_table_offset_buf) as usize;

        // Bounds-check the file table region before slicing — a malformed THOR
        // header (offset/length past EOF) must not panic.
        let table_end = file_table_offset
            .checked_add(file_table_compressed_len)
            .ok_or(Error::InvalidThorHeader)?;
        if table_end > data.len() {
            return Err(Error::InvalidThorHeader);
        }
        let compressed_table_data = &data[file_table_offset..table_end];
        
        let mut decoder = ZlibDecoder::new(compressed_table_data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| Error::Decompression(e.to_string()))?;
        
        let mut entries = Vec::new();
        let mut table_cursor = std::io::Cursor::new(decompressed);
        
        while table_cursor.position() < table_cursor.get_ref().len() as u64 {
            let mut filename_len_buf = [0u8; 1];
            if table_cursor.read(&mut filename_len_buf).unwrap_or(0) == 0 {
                break;
            }
            let filename_len = filename_len_buf[0] as usize;
            
            if filename_len == 0 {
                break;
            }
            
            let mut filename_buf = vec![0u8; filename_len];
            table_cursor.read_exact(&mut filename_buf)?;
            let filename = String::from_utf8_lossy(&filename_buf).to_string();
            
            let mut flags_buf = [0u8; 1];
            table_cursor.read_exact(&mut flags_buf)?;
            let flags = flags_buf[0];
            
            match flags {
                0x00 => {
                    let mut offset_buf = [0u8; 4];
                    table_cursor.read_exact(&mut offset_buf)?;
                    let offset = u32::from_le_bytes(offset_buf) as usize;
                    
                    let mut compressed_size_buf = [0u8; 4];
                    table_cursor.read_exact(&mut compressed_size_buf)?;
                    let compressed_size = u32::from_le_bytes(compressed_size_buf) as usize;
                    
                    let mut decompressed_size_buf = [0u8; 4];
                    table_cursor.read_exact(&mut decompressed_size_buf)?;
                    let _decompressed_size = u32::from_le_bytes(decompressed_size_buf) as usize;
                    
                    if offset + compressed_size <= data.len() {
                        let compressed_data = &data[offset..offset + compressed_size];
                        
                        let mut decoder = ZlibDecoder::new(compressed_data);
                        let mut decompressed = Vec::new();
                        match decoder.read_to_end(&mut decompressed) {
                            Ok(_) => {
                                entries.push(ThorEntry::Add { 
                                    filename, 
                                    data: decompressed
                                });
                            }
                            Err(_) => {
                                entries.push(ThorEntry::Add { 
                                    filename, 
                                    data: Vec::new()
                                });
                            }
                        }
                    } else {
                        entries.push(ThorEntry::Add { 
                            filename, 
                            data: Vec::new()
                        });
                    }
                },
                0x01 => {
                    entries.push(ThorEntry::Remove { filename });
                },
                _ => {},
            }
        }
        
        Ok(Thor { entries })
    }
    
    pub fn get_entries(&self) -> &[ThorEntry] {
        &self.entries
    }
    
    pub fn new() -> Self {
        Thor {
            entries: Vec::new(),
        }
    }

    pub fn add_file(&mut self, filename: &str, data: &[u8]) {
        self.entries.push(ThorEntry::Add {
            filename: filename.to_string(),
            data: data.to_vec(),
        });
    }
    
    pub fn remove_file(&mut self, filename: &str) {
        self.entries.push(ThorEntry::Remove {
            filename: filename.to_string(),
        });
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use flate2::Compression as FlateCompression;
        
        const HEADER_SIZE: u32 = 40;
        
        let mut file_data = Vec::new();
        let mut table_data = Vec::new();
        
        for entry in &self.entries {
            match entry {
                ThorEntry::Add { filename, data } => {
                    if filename.len() > 255 {
                        return Err(Error::Custom("Filename too long (max 255 bytes)".to_string()));
                    }
                    
                    let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::default());
                    encoder.write_all(data)?;
                    let compressed = encoder.finish()?;
                    
                    let offset = HEADER_SIZE + file_data.len() as u32;
                    let compressed_size = compressed.len() as u32;
                    let decompressed_size = data.len() as u32;
                    
                    file_data.write_all(&compressed)?;
                    
                    table_data.write_all(&[filename.len() as u8])?;
                    table_data.write_all(filename.as_bytes())?;
                    table_data.write_all(&[0x00])?;
                    table_data.write_all(&offset.to_le_bytes())?;
                    table_data.write_all(&compressed_size.to_le_bytes())?;
                    table_data.write_all(&decompressed_size.to_le_bytes())?;
                },
                ThorEntry::Remove { filename } => {
                    if filename.len() > 255 {
                        return Err(Error::Custom("Filename too long (max 255 bytes)".to_string()));
                    }
                    
                    table_data.write_all(&[filename.len() as u8])?;
                    table_data.write_all(filename.as_bytes())?;
                    table_data.write_all(&[0x01])?;
                },
            }
        }
        
        let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::default());
        encoder.write_all(&table_data)?;
        let compressed_table = encoder.finish()?;
        
        // THOR header (no target_grf payload): 24 magic + 1 use_grf + 4 num_files
        //                                    + 2 mode + 1 target_grf_len(=0) + 4 + 4 table fields
        let header_size = 24 + 1 + 4 + 2 + 1 + 4 + 4;
        let file_table_offset = header_size + file_data.len();
        let file_table_compressed_length = compressed_table.len() as u32;
        
        let mut file = std::fs::File::create(path)?;
        
        file.write_all(THOR_MAGIC)?;
        file.write_all(&[0x00])?;
        file.write_all(&(self.entries.len() as u32).to_le_bytes())?;
        file.write_all(&(0x30i16).to_le_bytes())?;
        file.write_all(&[0x00])?;
        file.write_all(&file_table_compressed_length.to_le_bytes())?;
        file.write_all(&(file_table_offset as u32).to_le_bytes())?;
        
        file.write_all(&file_data)?;
        file.write_all(&compressed_table)?;

        Ok(())
    }
}

impl Default for Thor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_truncated_header_without_panic() {
        // Shorter than the 24-byte magic.
        assert!(Thor::from_bytes(b"ASSF").is_err());
        // 24 bytes of magic but missing every following field.
        assert!(Thor::from_bytes(THOR_MAGIC).is_err());
    }

    #[test]
    fn rejects_out_of_bounds_file_table_offset() {
        // Craft a THOR with a sane magic but a file_table_offset past EOF.
        // Layout up to file_table_offset (using mode 0x30, target_grf_len=0):
        //   [24 magic][1 use_grf][4 num_files][2 mode][1 tg_len]
        //   [4 file_table_compressed_len][4 file_table_offset]
        let mut data = Vec::new();
        data.extend_from_slice(THOR_MAGIC);
        data.push(0x00);                                    // use_grf_merging
        data.extend_from_slice(&0u32.to_le_bytes());        // num_files
        data.extend_from_slice(&0x30i16.to_le_bytes());     // mode
        data.push(0x00);                                    // target_grf_len
        data.extend_from_slice(&16u32.to_le_bytes());       // compressed_len
        // Offset is past end of file → must be rejected, not panic.
        data.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());

        match Thor::from_bytes(&data) {
            Err(Error::InvalidThorHeader) => {}
            other => panic!("expected InvalidThorHeader, got {:?}", other),
        }
    }
}

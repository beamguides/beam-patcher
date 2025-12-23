use crate::{Error, Result};
use flate2::read::GzDecoder;
use std::io::Read;
use std::path::Path;

const THOR_MAGIC: &[u8; 28] = b"ASSF (C) 2007 Aeomin DEV\x1A\x04\x0C\x00";

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
        if data.len() < 28 {
            return Err(Error::InvalidThorHeader);
        }
        
        if &data[..28] != THOR_MAGIC {
            return Err(Error::InvalidThorHeader);
        }
        
        let compressed_data = &data[28..];
        
        let mut decoder = GzDecoder::new(compressed_data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| Error::Decompression(e.to_string()))?;
        
        let mut entries = Vec::new();
        let mut cursor = std::io::Cursor::new(decompressed);
        
        while cursor.position() < cursor.get_ref().len() as u64 {
            let mut mode_buf = [0u8; 1];
            if cursor.read(&mut mode_buf).unwrap_or(0) == 0 {
                break;
            }
            let mode = mode_buf[0];
            
            let mut filename_len_buf = [0u8; 1];
            cursor.read_exact(&mut filename_len_buf)?;
            let filename_len = filename_len_buf[0] as usize;
            
            let mut filename_buf = vec![0u8; filename_len];
            cursor.read_exact(&mut filename_buf)?;
            let filename = String::from_utf8_lossy(&filename_buf).to_string();
            
            match mode {
                0x01 => {
                    let mut size_buf = [0u8; 4];
                    cursor.read_exact(&mut size_buf)?;
                    let size = u32::from_le_bytes(size_buf);
                    
                    let mut data = vec![0u8; size as usize];
                    cursor.read_exact(&mut data)?;
                    
                    entries.push(ThorEntry::Add { filename, data });
                },
                0x02 => {
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
}

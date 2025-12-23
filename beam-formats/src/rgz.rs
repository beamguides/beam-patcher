use crate::{Error, Result};
use flate2::read::GzDecoder;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub enum RgzEntry {
    File {
        name: String,
        data: Vec<u8>,
    },
    Directory {
        name: String,
    },
}

#[derive(Debug)]
pub struct Rgz {
    pub entries: Vec<RgzEntry>,
}

impl Rgz {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_bytes(&data)
    }
    
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let mut decoder = GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)
            .map_err(|e| Error::Decompression(e.to_string()))?;
        
        let mut entries = Vec::new();
        let mut cursor = std::io::Cursor::new(decompressed);
        
        loop {
            let mut type_buf = [0u8; 1];
            if cursor.read(&mut type_buf).unwrap_or(0) == 0 {
                break;
            }
            let entry_type = type_buf[0];
            
            match entry_type {
                b'f' => {
                    let mut name_len_buf = [0u8; 1];
                    cursor.read_exact(&mut name_len_buf)?;
                    let name_len = name_len_buf[0] as usize;
                    
                    let mut name_buf = vec![0u8; name_len];
                    cursor.read_exact(&mut name_buf)?;
                    let name = String::from_utf8_lossy(&name_buf).to_string();
                    
                    let mut size_buf = [0u8; 4];
                    cursor.read_exact(&mut size_buf)?;
                    let size = u32::from_le_bytes(size_buf);
                    
                    let mut data = vec![0u8; size as usize];
                    cursor.read_exact(&mut data)?;
                    
                    entries.push(RgzEntry::File { name, data });
                },
                b'd' => {
                    let mut name_len_buf = [0u8; 1];
                    cursor.read_exact(&mut name_len_buf)?;
                    let name_len = name_len_buf[0] as usize;
                    
                    let mut name_buf = vec![0u8; name_len];
                    cursor.read_exact(&mut name_buf)?;
                    let name = String::from_utf8_lossy(&name_buf).to_string();
                    
                    entries.push(RgzEntry::Directory { name });
                },
                b'e' => break,
                _ => return Err(Error::InvalidRgzFormat),
            }
        }
        
        Ok(Rgz { entries })
    }
    
    pub fn get_entries(&self) -> &[RgzEntry] {
        &self.entries
    }
}

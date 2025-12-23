use crate::grf::{Grf, GrfEntry};
use crate::Result;
use std::path::Path;

pub struct Gpf {
    grf: Grf,
}

impl Gpf {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let grf = Grf::open(path)?;
        Ok(Gpf { grf })
    }
    
    pub fn get_entry(&self, filename: &str) -> Option<&GrfEntry> {
        self.grf.get_entry(filename)
    }
    
    pub fn extract_file(&self, filename: &str) -> Result<Vec<u8>> {
        self.grf.extract_file(filename)
    }
    
    pub fn list_files(&self) -> Vec<&str> {
        self.grf.list_files()
    }
    
    pub fn file_count(&self) -> usize {
        self.grf.file_count()
    }
}

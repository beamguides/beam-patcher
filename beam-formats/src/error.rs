use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid GRF header")]
    InvalidGrfHeader,
    
    #[error("Invalid GRF version: {0:#x}")]
    InvalidGrfVersion(u32),
    
    #[error("Invalid THOR header")]
    InvalidThorHeader,
    
    #[error("Invalid RGZ format")]
    InvalidRgzFormat,
    
    #[error("Decompression error: {0}")]
    Decompression(String),
    
    #[error("Compression error: {0}")]
    Compression(String),
    
    #[error("Decryption error")]
    Decryption,
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Invalid file entry")]
    InvalidFileEntry,
    
    #[error("Unsupported operation: {0}")]
    Unsupported(String),
    
    #[error("{0}")]
    Custom(String),
}

pub type Result<T> = std::result::Result<T, Error>;

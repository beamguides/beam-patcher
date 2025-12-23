use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Format error: {0}")]
    Format(#[from] beam_formats::Error),
    
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Config error: {0}")]
    Config(#[from] serde_yaml::Error),
    
    #[error("Self update error: {0}")]
    SelfUpdate(String),
    
    #[error("Download failed: {0}")]
    DownloadFailed(String),
    
    #[error("Patch failed: {0}")]
    PatchFailed(String),
    
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
    
    #[error("Update failed: {0}")]
    UpdateFailed(String),
    
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for Error {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Error::SelfUpdate(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

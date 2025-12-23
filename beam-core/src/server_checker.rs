use crate::{Config, Result};
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use std::time::Duration;
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatusResult {
    pub login_online: bool,
    pub char_online: bool,
    pub map_online: bool,
}

pub struct ServerChecker {
    config: Config,
}

impl ServerChecker {
    pub fn new(config: Config) -> Self {
        ServerChecker { config }
    }
    
    pub async fn check_servers(&self) -> Result<ServerStatusResult> {
        let server_config = self.config.server.as_ref()
            .ok_or_else(|| crate::Error::InvalidConfig("Server configuration not found".to_string()))?;
        
        let login_online = self.check_tcp_connection(
            &server_config.login_server_ip,
            server_config.login_server_port
        ).await;
        
        let char_online = self.check_tcp_connection(
            &server_config.char_server_ip,
            server_config.char_server_port
        ).await;
        
        let map_online = self.check_tcp_connection(
            &server_config.map_server_ip,
            server_config.map_server_port
        ).await;
        
        Ok(ServerStatusResult {
            login_online,
            char_online,
            map_online,
        })
    }
    
    async fn check_tcp_connection(&self, ip: &str, port: u16) -> bool {
        let address = format!("{}:{}", ip, port);
        debug!("Checking connection to {}", address);
        
        match TcpStream::connect_timeout(
            &address.parse().unwrap(),
            Duration::from_secs(10)
        ) {
            Ok(_) => {
                debug!("Successfully connected to {}", address);
                true
            }
            Err(e) => {
                warn!("Failed to connect to {}: {}", address, e);
                false
            }
        }
    }
}

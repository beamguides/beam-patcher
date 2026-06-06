use crate::{Config, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::net::lookup_host;
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

        let (login_online, char_online, map_online) = tokio::join!(
            Self::check_tcp_connection(&server_config.login_server_ip, server_config.login_server_port),
            Self::check_tcp_connection(&server_config.char_server_ip, server_config.char_server_port),
            Self::check_tcp_connection(&server_config.map_server_ip, server_config.map_server_port),
        );

        Ok(ServerStatusResult {
            login_online,
            char_online,
            map_online,
        })
    }

    async fn check_tcp_connection(host: &str, port: u16) -> bool {
        let address = format!("{}:{}", host, port);
        debug!("Checking connection to {}", address);

        let mut addrs = match lookup_host(&address).await {
            Ok(a) => a,
            Err(e) => {
                warn!("DNS lookup failed for {}: {}", address, e);
                return false;
            }
        };

        let Some(socket_addr) = addrs.next() else {
            warn!("No addresses resolved for {}", address);
            return false;
        };

        match tokio::time::timeout(
            Duration::from_secs(8),
            TcpStream::connect(&socket_addr),
        ).await {
            Ok(Ok(_)) => {
                debug!("Successfully connected to {}", address);
                true
            }
            Ok(Err(e)) => {
                warn!("Failed to connect to {}: {}", address, e);
                false
            }
            Err(_) => {
                warn!("Timeout connecting to {}", address);
                false
            }
        }
    }
}

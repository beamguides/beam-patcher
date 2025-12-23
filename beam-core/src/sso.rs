use crate::{Config, Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub refresh_token: Option<String>,
}

pub struct SsoClient {
    config: Config,
    client: Client,
}

impl SsoClient {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .user_agent("Beam-Patcher/1.0")
            .build()?;
        
        Ok(SsoClient { config, client })
    }
    
    pub fn get_login_url(&self) -> Result<String> {
        let sso_config = self.config.sso.as_ref()
            .ok_or_else(|| Error::InvalidConfig("SSO not configured".to_string()))?;
        
        if !sso_config.enabled {
            return Err(Error::AuthFailed("SSO is disabled".to_string()));
        }
        
        let url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code",
            sso_config.login_url,
            sso_config.client_id,
            urlencoding::encode(&sso_config.redirect_uri)
        );
        
        Ok(url)
    }
    
    pub async fn exchange_code_for_token(&self, code: &str) -> Result<TokenResponse> {
        let sso_config = self.config.sso.as_ref()
            .ok_or_else(|| Error::InvalidConfig("SSO not configured".to_string()))?;
        
        info!("Exchanging authorization code for token");
        
        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", &sso_config.client_id),
            ("redirect_uri", &sso_config.redirect_uri),
        ];
        
        let response = self.client
            .post(&sso_config.token_url)
            .form(&params)
            .send()
            .await?;
        
        if !response.status().is_success() {
            return Err(Error::AuthFailed(format!(
                "Token exchange failed: HTTP {}",
                response.status()
            )));
        }
        
        let token_response: TokenResponse = response.json().await?;
        info!("Successfully obtained access token");
        
        Ok(token_response)
    }
    
    pub async fn launch_game(&self, token: &str, executable: &str) -> Result<()> {
        info!("Launching game with SSO token");
        
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new(executable)
                .arg(format!("-token:{}", token))
                .spawn()?;
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            std::process::Command::new(executable)
                .arg(format!("--token={}", token))
                .spawn()?;
        }
        
        Ok(())
    }
}

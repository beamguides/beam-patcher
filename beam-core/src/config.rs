use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    pub patcher: PatcherConfig,
    pub ui: UiConfig,
    pub sso: Option<SsoConfig>,
    pub updater: Option<UpdaterConfig>,
    pub server: Option<ServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub version: String,
    pub window_title: String,
    pub game_directory: Option<String>,
    pub client_exe: String,
    pub setup_exe: Option<String>,
    pub bgm_autoplay: Option<bool>,
    pub bgm_file: Option<String>,
    pub server_name: Option<String>,
    pub video_background_enabled: Option<bool>,
    pub video_background_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatcherConfig {
    pub mirrors: Vec<MirrorConfig>,
    pub patch_list_url: String,
    pub target_grf: String,
    pub allow_manual_patch: bool,
    pub verify_checksums: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorConfig {
    pub name: String,
    pub url: String,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub custom_css: Option<String>,
    pub logo: Option<String>,
    pub background: Option<String>,
    pub show_progress: bool,
    pub show_file_list: bool,
    pub news_feed_url: Option<String>,
    pub server_status_url: Option<String>,
    pub custom_buttons: Vec<CustomButton>,
    pub layout: LayoutConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomButton {
    pub label: String,
    pub image: Option<String>,
    pub url: String,
    pub position: ButtonPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonPosition {
    pub x: String,
    pub y: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub width: u32,
    pub height: u32,
    pub use_custom_layout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    pub enabled: bool,
    pub login_url: String,
    pub token_url: String,
    pub client_id: String,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdaterConfig {
    pub enabled: bool,
    pub check_url: String,
    pub update_url: String,
    pub auto_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub login_server_ip: String,
    pub login_server_port: u16,
    pub char_server_ip: String,
    pub char_server_port: u16,
    pub map_server_ip: String,
    pub map_server_port: u16,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_yaml::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.patcher.mirrors.is_empty() {
            return Err(Error::InvalidConfig("At least one mirror must be configured".to_string()));
        }
        
        if self.patcher.patch_list_url.is_empty() {
            return Err(Error::InvalidConfig("Patch list URL cannot be empty".to_string()));
        }
        
        if self.patcher.target_grf.is_empty() {
            return Err(Error::InvalidConfig("Target GRF cannot be empty".to_string()));
        }
        
        Ok(())
    }
    
    pub fn default() -> Self {
        Config {
            app: AppConfig {
                name: "Beam Patcher".to_string(),
                version: "1.0.0".to_string(),
                window_title: "Beam Patcher - Modern RO Patcher".to_string(),
                game_directory: None,
                client_exe: "Ragnarok.exe".to_string(),
                setup_exe: Some("setup.exe".to_string()),
                bgm_autoplay: Some(false),
                bgm_file: None,
                server_name: Some("MyRO".to_string()),
                video_background_enabled: Some(false),
                video_background_file: None,
            },
            patcher: PatcherConfig {
                mirrors: vec![
                    MirrorConfig {
                        name: "Primary Mirror".to_string(),
                        url: "https://patch.example.com".to_string(),
                        priority: 1,
                    },
                ],
                patch_list_url: "https://patch.example.com/patchlist.txt".to_string(),
                target_grf: "data.grf".to_string(),
                allow_manual_patch: true,
                verify_checksums: true,
            },
            ui: UiConfig {
                theme: "default".to_string(),
                custom_css: None,
                logo: None,
                background: None,
                show_progress: true,
                show_file_list: true,
                news_feed_url: None,
                server_status_url: None,
                custom_buttons: vec![],
                layout: LayoutConfig {
                    width: 800,
                    height: 600,
                    use_custom_layout: false,
                },
            },
            sso: Some(SsoConfig {
                enabled: false,
                login_url: "https://auth.example.com/login".to_string(),
                token_url: "https://auth.example.com/token".to_string(),
                client_id: "beam-patcher".to_string(),
                redirect_uri: "http://localhost:8080/callback".to_string(),
            }),
            updater: Some(UpdaterConfig {
                enabled: true,
                check_url: "https://patch.example.com/version.json".to_string(),
                update_url: "https://patch.example.com/updates".to_string(),
                auto_update: false,
            }),
            server: Some(ServerConfig {
                login_server_ip: "127.0.0.1".to_string(),
                login_server_port: 6900,
                char_server_ip: "127.0.0.1".to_string(),
                char_server_port: 6121,
                map_server_ip: "127.0.0.1".to_string(),
                map_server_port: 5121,
            }),
        }
    }
}

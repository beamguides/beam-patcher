use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    pub resolution_width: u32,
    pub resolution_height: u32,
    pub fullscreen: bool,
    pub sound_enabled: bool,
    pub bgm_enabled: bool,
    pub mouse_freedom: bool,
    pub vsync: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            resolution_width: 1920,
            resolution_height: 1080,
            fullscreen: false,
            sound_enabled: true,
            bgm_enabled: true,
            mouse_freedom: true,
            vsync: true,
        }
    }
}

pub struct GameSettingsManager {
    game_directory: PathBuf,
}

impl GameSettingsManager {
    pub fn new<P: AsRef<Path>>(game_directory: P) -> Self {
        GameSettingsManager {
            game_directory: game_directory.as_ref().to_path_buf(),
        }
    }
    
    pub fn apply_settings(&self, settings: &GameSettings) -> Result<()> {
        info!("Applying game settings to {:?}", self.game_directory);
        
        let data_ini_path = self.game_directory.join("DATA.INI");
        
        if data_ini_path.exists() {
            self.modify_data_ini(&data_ini_path, settings)?;
        } else {
            self.create_data_ini(&data_ini_path, settings)?;
        }
        
        #[cfg(target_os = "windows")]
        self.apply_registry_settings(settings)?;
        
        Ok(())
    }
    
    fn modify_data_ini(&self, path: &Path, settings: &GameSettings) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        
        self.update_ini_value(&mut lines, "WIDTH", &settings.resolution_width.to_string());
        self.update_ini_value(&mut lines, "HEIGHT", &settings.resolution_height.to_string());
        self.update_ini_value(&mut lines, "SCREENMODE", if settings.fullscreen { "1" } else { "0" });
        self.update_ini_value(&mut lines, "SOUNDVOLUME", if settings.sound_enabled { "100" } else { "0" });
        self.update_ini_value(&mut lines, "BGMVOLUME", if settings.bgm_enabled { "100" } else { "0" });
        self.update_ini_value(&mut lines, "ISFULLSCREENMODE", if settings.fullscreen { "1" } else { "0" });
        self.update_ini_value(&mut lines, "MOUSEEXCLUSIVE", if settings.mouse_freedom { "0" } else { "1" });
        
        let new_content = lines.join("\n");
        fs::write(path, new_content)?;
        
        info!("DATA.INI updated successfully");
        Ok(())
    }
    
    fn create_data_ini(&self, path: &Path, settings: &GameSettings) -> Result<()> {
        let content = format!(
            "[SETTING]\n\
             WIDTH={}\n\
             HEIGHT={}\n\
             SCREENMODE={}\n\
             ISFULLSCREENMODE={}\n\
             SOUNDVOLUME={}\n\
             BGMVOLUME={}\n\
             MOUSEEXCLUSIVE={}\n\
             SPRITE=3\n\
             TEXTURE=3\n\
             DIGITAL=0\n",
            settings.resolution_width,
            settings.resolution_height,
            if settings.fullscreen { 1 } else { 0 },
            if settings.fullscreen { 1 } else { 0 },
            if settings.sound_enabled { 100 } else { 0 },
            if settings.bgm_enabled { 100 } else { 0 },
            if settings.mouse_freedom { 0 } else { 1 }
        );
        
        fs::write(path, content)?;
        info!("DATA.INI created successfully");
        Ok(())
    }
    
    fn update_ini_value(&self, lines: &mut Vec<String>, key: &str, value: &str) {
        let key_prefix = format!("{}=", key);
        
        for line in lines.iter_mut() {
            if line.starts_with(&key_prefix) {
                *line = format!("{}={}", key, value);
                return;
            }
        }
        
        lines.push(format!("{}={}", key, value));
    }
    
    #[cfg(target_os = "windows")]
    fn apply_registry_settings(&self, settings: &GameSettings) -> Result<()> {
        use winreg::enums::*;
        use winreg::RegKey;
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        
        match hkcu.create_subkey(r"Software\Gravity Soft\Ragnarok") {
            Ok((key, _)) => {
                let _ = key.set_value("Width", &settings.resolution_width);
                let _ = key.set_value("Height", &settings.resolution_height);
                let _ = key.set_value("Fullscreen", &(if settings.fullscreen { 1u32 } else { 0u32 }));
                let _ = key.set_value("SoundVolume", &(if settings.sound_enabled { 100u32 } else { 0u32 }));
                let _ = key.set_value("BGMVolume", &(if settings.bgm_enabled { 100u32 } else { 0u32 }));
                
                info!("Registry settings applied successfully");
            }
            Err(e) => {
                warn!("Failed to apply registry settings: {}", e);
            }
        }
        
        Ok(())
    }
    
    #[cfg(not(target_os = "windows"))]
    fn apply_registry_settings(&self, _settings: &GameSettings) -> Result<()> {
        Ok(())
    }
    
    pub fn load_settings(&self) -> Result<GameSettings> {
        let data_ini_path = self.game_directory.join("DATA.INI");
        
        if data_ini_path.exists() {
            return self.load_from_data_ini(&data_ini_path);
        }
        
        let setup_exe_path = self.game_directory.join("opensetup.exe");
        if setup_exe_path.exists() {
            info!("DATA.INI not found, attempting to read from opensetup.exe");
            return self.load_from_setup_exe(&setup_exe_path);
        }
        
        Ok(GameSettings::default())
    }
    
    fn load_from_data_ini(&self, path: &Path) -> Result<GameSettings> {
        let content = fs::read_to_string(path)?;
        let mut settings = GameSettings::default();
        
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "WIDTH" => settings.resolution_width = value.trim().parse().unwrap_or(1920),
                    "HEIGHT" => settings.resolution_height = value.trim().parse().unwrap_or(1080),
                    "SCREENMODE" | "ISFULLSCREENMODE" => {
                        settings.fullscreen = value.trim() == "1";
                    }
                    "SOUNDVOLUME" => {
                        settings.sound_enabled = value.trim().parse::<u32>().unwrap_or(0) > 0;
                    }
                    "BGMVOLUME" => {
                        settings.bgm_enabled = value.trim().parse::<u32>().unwrap_or(0) > 0;
                    }
                    "MOUSEEXCLUSIVE" => {
                        settings.mouse_freedom = value.trim() == "0";
                    }
                    _ => {}
                }
            }
        }
        
        Ok(settings)
    }
    
    fn load_from_setup_exe(&self, _path: &Path) -> Result<GameSettings> {
        #[cfg(target_os = "windows")]
        {
            use winreg::enums::*;
            use winreg::RegKey;
            
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let mut settings = GameSettings::default();
            
            if let Ok(key) = hkcu.open_subkey(r"Software\Gravity Soft\Ragnarok") {
                if let Ok(width) = key.get_value::<u32, _>("Width") {
                    settings.resolution_width = width;
                }
                if let Ok(height) = key.get_value::<u32, _>("Height") {
                    settings.resolution_height = height;
                }
                if let Ok(fullscreen) = key.get_value::<u32, _>("Fullscreen") {
                    settings.fullscreen = fullscreen == 1;
                }
                if let Ok(sound) = key.get_value::<u32, _>("SoundVolume") {
                    settings.sound_enabled = sound > 0;
                }
                if let Ok(bgm) = key.get_value::<u32, _>("BGMVolume") {
                    settings.bgm_enabled = bgm > 0;
                }
                
                info!("Loaded settings from Windows Registry (opensetup.exe fallback)");
            } else {
                warn!("Could not read registry settings, using defaults");
            }
            
            Ok(settings)
        }
        
        #[cfg(not(target_os = "windows"))]
        {
            Ok(GameSettings::default())
        }
    }
}

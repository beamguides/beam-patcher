use beam_core::{Config, Patcher, Verifier, VerificationResult, GameSettings, GameSettingsManager, ServerChecker, ServerStatusResult, ClientChecker, ClientStatusResult};
use serde::{Deserialize, Serialize};
use tauri::{State, AppHandle};
use crate::{AppState, PatchProgress};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    pub date: String,
    pub category: String,
}



#[tauri::command]
pub async fn start_patching(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let patcher = Patcher::new(config).map_err(|e| e.to_string())?;
    
    patcher.run_full_patch().await.map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn check_updates(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let config = state.config.lock().unwrap().clone();
    let updater = beam_core::Updater::new(config).map_err(|e| e.to_string())?;
    
    let update_info = updater.check_for_updates().await.map_err(|e| e.to_string())?;
    
    Ok(update_info.map(|info| info.version))
}

#[tauri::command]
pub async fn perform_update(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let updater = beam_core::Updater::new(config).map_err(|e| e.to_string())?;
    
    let update_info = updater.check_for_updates().await.map_err(|e| e.to_string())?;
    
    if let Some(info) = update_info {
        updater.perform_update(&info).await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_login_url(state: State<'_, AppState>) -> Result<String, String> {
    let config = state.config.lock().unwrap().clone();
    let sso_client = beam_core::SsoClient::new(config).map_err(|e| e.to_string())?;
    
    sso_client.get_login_url().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn exchange_sso_code(
    state: State<'_, AppState>,
    code: String,
) -> Result<String, String> {
    let config = state.config.lock().unwrap().clone();
    let sso_client = beam_core::SsoClient::new(config).map_err(|e| e.to_string())?;
    
    let token_response = sso_client
        .exchange_code_for_token(&code)
        .await
        .map_err(|e| e.to_string())?;
    
    Ok(token_response.access_token)
}

#[tauri::command]
pub async fn launch_game(
    state: State<'_, AppState>,
    _token: String,
) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    
    let game_dir = config.app.game_directory
        .ok_or("Game directory not set. Please select game folder first.")?;
    
    let client_exe = PathBuf::from(&game_dir).join(&config.app.client_exe);
    
    if !client_exe.exists() {
        return Err(format!(
            "Game executable not found: {}. Please check your game directory.",
            client_exe.display()
        ));
    }
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new(&client_exe)
            .current_dir(&game_dir)
            .spawn()
            .map_err(|e| format!("Failed to launch game: {}", e))?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        return Err("Game launch is only supported on Windows".to_string());
    }
    
    Ok(())
}

#[tauri::command]
pub fn get_config(state: State<'_, AppState>) -> Result<Config, String> {
    let config = state.config.lock().unwrap().clone();
    Ok(config)
}

#[tauri::command]
pub fn get_progress(state: State<'_, AppState>) -> Result<PatchProgress, String> {
    let progress = state.progress.lock().unwrap().clone();
    Ok(progress)
}

#[tauri::command]
pub async fn get_news(state: State<'_, AppState>) -> Result<Vec<NewsItem>, String> {
    let config = state.config.lock().unwrap().clone();
    
    if let Some(news_url) = config.ui.news_feed_url {
        let client = reqwest::Client::new();
        let response = client.get(&news_url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        let news: Vec<NewsItem> = response.json()
            .await
            .map_err(|e| e.to_string())?;
        
        Ok(news)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
pub async fn get_server_status(state: State<'_, AppState>) -> Result<ServerStatusResult, String> {
    let config = state.config.lock().unwrap().clone();
    
    let checker = ServerChecker::new(config);
    checker.check_servers().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn verify_game_files(state: State<'_, AppState>) -> Result<VerificationResult, String> {
    let config = state.config.lock().unwrap().clone();
    
    let manifest_url = format!(
        "{}/manifest.json",
        config.patcher.mirrors.first()
            .map(|m| m.url.as_str())
            .unwrap_or("https://patch.example.com")
    );
    
    let verifier = Verifier::new(config, manifest_url).map_err(|e| e.to_string())?;
    
    verifier.verify_game_files().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_game_directory(
    state: State<'_, AppState>,
    directory: String,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.app.game_directory = Some(directory.clone());
    
    config.save("config.yml").map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn get_game_directory(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let config = state.config.lock().unwrap();
    Ok(config.app.game_directory.clone())
}

#[tauri::command]
pub async fn get_client_status(state: State<'_, AppState>) -> Result<ClientStatusResult, String> {
    let config = state.config.lock().unwrap().clone();
    
    let checker = ClientChecker::new(config);
    checker.check_client_integrity().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_game_settings(
    state: State<'_, AppState>,
    settings: GameSettings,
) -> Result<(), String> {
    let config = state.config.lock().unwrap();
    
    let game_dir = config.app.game_directory.as_ref()
        .ok_or("Game directory not set")?;
    
    let manager = GameSettingsManager::new(game_dir);
    manager.apply_settings(&settings).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub async fn load_game_settings(state: State<'_, AppState>) -> Result<GameSettings, String> {
    let config = state.config.lock().unwrap();
    
    let game_dir = config.app.game_directory.as_ref()
        .ok_or("Game directory not set")?;
    
    let manager = GameSettingsManager::new(game_dir);
    manager.load_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resolve_resource_path(app: AppHandle, path: String) -> Result<String, String> {
    let resource_path = app.path_resolver()
        .resolve_resource(&path)
        .ok_or(format!("Failed to resolve resource path: {}", path))?;
    
    Ok(resource_path.to_string_lossy().to_string())
}

use beam_core::{Config, Patcher, Verifier, VerificationResult, GameSettings, GameSettingsManager, ServerChecker, ServerStatusResult, ClientChecker, ClientStatusResult};
use serde::{Deserialize, Serialize};
use tauri::{State, AppHandle, Manager};
use crate::{AppState, PatchProgress};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub id: Option<i32>,
    pub title: String,
    pub content: Option<String>,
    pub author: Option<String>,
    pub category: String,
    pub image_url: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub published: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsApiResponse {
    pub success: bool,
    pub data: Vec<NewsItem>,
}



#[tauri::command]
pub async fn check_patches_available(state: State<'_, AppState>) -> Result<usize, String> {
    let config = state.config.lock().unwrap().clone();
    let patcher = Patcher::new(config).map_err(|e| e.to_string())?;
    
    let patch_count = patcher.check_available_patches().await.map_err(|e| e.to_string())?;
    
    Ok(patch_count)
}

#[tauri::command]
pub async fn start_patching(state: State<'_, AppState>, app: AppHandle) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();

    let progress_state = state.progress.clone();

    {
        let mut progress = progress_state.lock().unwrap();
        progress.status = "Initializing...".to_string();
        progress.current = 0;
        progress.total = 0;
        progress.bytes_downloaded = 0;
        progress.bytes_total = 0;
    }

    let patcher = Patcher::new(config).map_err(|e| e.to_string())?;

    let patches = patcher.get_patch_list().await.map_err(|e| e.to_string())?;
    let total_patches = patches.len();
    let filenames: Vec<String> = patches.iter().map(|p| p.filename.clone()).collect();

    {
        let mut progress = progress_state.lock().unwrap();
        progress.total = total_patches;
        progress.status = format!("Found {} patches to download", total_patches);
    }

    let snapshot = progress_state.lock().unwrap().clone();
    app.emit_all("patch-progress", snapshot)
        .map_err(|e: tauri::Error| e.to_string())?;

    let progress_clone = progress_state.clone();
    let app_clone = app.clone();

    // Batched path: GRF is opened once and rebuilt once at the end (huge win
    // on multi-patch updates over a large data.grf).
    patcher
        .download_and_apply_patches(&patches, move |idx, total, downloaded, bytes_total| {
            let mut progress = progress_clone.lock().unwrap();
            let fname = filenames.get(idx).cloned().unwrap_or_default();
            progress.current = idx + 1;
            progress.filename = fname.clone();
            progress.bytes_downloaded = downloaded;
            progress.bytes_total = bytes_total;
            progress.status = format!("Patch {}/{}: {}", idx + 1, total, fname);
            let snapshot = progress.clone();
            drop(progress);
            let _ = app_clone.emit_all("patch-progress", snapshot);
        })
        .await
        .map_err(|e| e.to_string())?;

    {
        let mut progress = progress_state.lock().unwrap();
        progress.status = "Patching complete!".to_string();
        progress.current = total_patches;
    }

    let snapshot = progress_state.lock().unwrap().clone();
    app.emit_all("patch-progress", snapshot)
        .map_err(|e: tauri::Error| e.to_string())?;

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
    
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();
    
    let (client_exe, working_dir) = if let Some(game_dir) = &config.app.game_directory {
        let client_path = PathBuf::from(game_dir).join(&config.app.client_exe);
        if client_path.exists() {
            (client_path, PathBuf::from(game_dir))
        } else {
            let local_client = exe_dir.join(&config.app.client_exe);
            if local_client.exists() {
                (local_client.clone(), exe_dir.clone())
            } else {
                return Err(format!(
                    "Game executable not found. Searched in:\n- {}\n- {}",
                    client_path.display(),
                    local_client.display()
                ));
            }
        }
    } else {
        let local_client = exe_dir.join(&config.app.client_exe);
        if local_client.exists() {
            (local_client.clone(), exe_dir.clone())
        } else {
            return Err(format!(
                "Game executable not found: {}. Please set game directory or place the executable in the patcher folder.",
                local_client.display()
            ));
        }
    };
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new(&client_exe)
            .current_dir(&working_dir)
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
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        
        let response = client.get(&news_url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch news: {}", e))?;
        
        let api_response: NewsApiResponse = response.json()
            .await
            .map_err(|e| format!("Failed to parse news: {}", e))?;
        
        if api_response.success {
            Ok(api_response.data)
        } else {
            Ok(vec![])
        }
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
pub async fn select_game_directory(state: State<'_, AppState>) -> Result<String, String> {
    let selected = tauri::api::dialog::blocking::FileDialogBuilder::new()
        .set_title("Select Game Directory")
        .pick_folder()
        .ok_or("No directory selected")?;
    
    let directory_path = selected.to_string_lossy().to_string();
    
    let mut config = state.config.lock().unwrap();
    config.app.game_directory = Some(directory_path.clone());
    
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();
    
    let config_path = exe_dir.join("config.yml");
    config.save(&config_path).map_err(|e| e.to_string())?;
    
    Ok(directory_path)
}

#[tauri::command]
pub async fn set_game_directory(
    state: State<'_, AppState>,
    directory: String,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();
    config.app.game_directory = Some(directory.clone());
    
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();
    
    let config_path = exe_dir.join("config.yml");
    
    config.save(&config_path).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
pub fn check_initial_setup(state: State<'_, AppState>) -> Result<bool, String> {
    let config = state.config.lock().unwrap();
    Ok(config.app.game_directory.is_none())
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
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();
    
    let local_path = exe_dir.join(&path);
    
    if local_path.exists() {
        Ok(local_path.to_string_lossy().to_string())
    } else {
        let resource_path = app.path_resolver()
            .resolve_resource(&path)
            .ok_or(format!("Failed to resolve resource path: {}. Make sure the file exists in the patcher directory.", path))?;
        
        Ok(resource_path.to_string_lossy().to_string())
    }
}

#[tauri::command]
pub async fn open_setup(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    
    let setup_exe_name = config.app.setup_exe
        .ok_or("Setup executable not configured")?;
    
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?
        .parent()
        .ok_or("Failed to get executable directory")?
        .to_path_buf();
    
    let (setup_exe_path, working_dir) = if let Some(game_dir) = &config.app.game_directory {
        let setup_path = PathBuf::from(game_dir).join(&setup_exe_name);
        if setup_path.exists() {
            (setup_path, PathBuf::from(game_dir))
        } else {
            let local_setup = exe_dir.join(&setup_exe_name);
            if local_setup.exists() {
                (local_setup.clone(), exe_dir.clone())
            } else {
                return Err(format!(
                    "Setup executable not found. Searched in:\n- {}\n- {}",
                    setup_path.display(),
                    local_setup.display()
                ));
            }
        }
    } else {
        let local_setup = exe_dir.join(&setup_exe_name);
        if local_setup.exists() {
            (local_setup.clone(), exe_dir.clone())
        } else {
            return Err(format!(
                "Setup executable not found: {}. Please set game directory or place the executable in the patcher folder.",
                local_setup.display()
            ));
        }
    };
    
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new(&setup_exe_path)
            .current_dir(&working_dir)
            .spawn()
            .map_err(|e| format!("Failed to launch setup: {}", e))?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        return Err("Setup launch is only supported on Windows".to_string());
    }
    
    Ok(())
}

#[tauri::command]
pub async fn manual_patch(state: State<'_, AppState>) -> Result<(), String> {
    let selected = tauri::api::dialog::blocking::FileDialogBuilder::new()
        .add_filter("Patch Files", &["gpf", "patch", "grf"])
        .pick_file()
        .ok_or("No file selected")?;
    
    let config = state.config.lock().unwrap().clone();
    let game_dir = config.app.game_directory
        .ok_or("Game directory not set")?;
    
    let target_dir = PathBuf::from(&game_dir);
    let file_name = selected.file_name()
        .ok_or("Invalid file name")?;
    
    let destination = target_dir.join(file_name);
    
    std::fs::copy(&selected, &destination)
        .map_err(|e| format!("Failed to copy patch file: {}", e))?;
    
    Ok(())
}

#[tauri::command]
pub async fn reset_cache(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let downloader = beam_core::Downloader::new(config).map_err(|e| e.to_string())?;
    
    downloader.clear_cache().map_err(|e| e.to_string())?;
    
    Ok(())
}

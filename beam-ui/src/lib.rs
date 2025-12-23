mod commands;

use anyhow::Result;
use beam_core::Config;
use beam_core::Patcher;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchProgress {
    pub current: usize,
    pub total: usize,
    pub filename: String,
    pub bytes_downloaded: u64,
    pub bytes_total: u64,
    pub status: String,
}

pub struct AppState {
    pub config: Arc<Mutex<Config>>,
    pub patcher: Arc<Mutex<Option<Patcher>>>,
    pub progress: Arc<Mutex<PatchProgress>>,
}

pub fn run_ui(config: Config) -> Result<()> {
    let app_state = AppState {
        config: Arc::new(Mutex::new(config.clone())),
        patcher: Arc::new(Mutex::new(None)),
        progress: Arc::new(Mutex::new(PatchProgress {
            current: 0,
            total: 0,
            filename: String::new(),
            bytes_downloaded: 0,
            bytes_total: 0,
            status: "Idle".to_string(),
        })),
    };
    
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::start_patching,
            commands::check_updates,
            commands::perform_update,
            commands::get_login_url,
            commands::exchange_sso_code,
            commands::launch_game,
            commands::get_config,
            commands::get_progress,
            commands::get_news,
            commands::get_server_status,
            commands::get_client_status,
            commands::verify_game_files,
            commands::set_game_directory,
            commands::get_game_directory,
            commands::apply_game_settings,
            commands::load_game_settings,
            commands::resolve_resource_path,
        ])
        .run(tauri::generate_context!("tauri.conf.json"))
        .expect("error while running tauri application");
    
    Ok(())
}

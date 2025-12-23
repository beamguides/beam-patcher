#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use beam_core::Config;
use beam_ui;
use clap::Parser;
use tracing_subscriber;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "config.yml")]
    config: String,
    
    #[arg(long)]
    headless: bool,
    
    #[arg(short, long)]
    manual_patch: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let args = Args::parse();
    
    let config = if std::path::Path::new(&args.config).exists() {
        Config::load(&args.config)?
    } else {
        tracing::warn!("Config file not found, creating default config");
        let config = Config::default();
        config.save(&args.config)?;
        config
    };
    
    if args.headless {
        run_headless(config, args.manual_patch).await?;
    } else {
        beam_ui::run_ui(config)?;
    }
    
    Ok(())
}

async fn run_headless(config: Config, manual_patch: Option<String>) -> Result<()> {
    let patcher = beam_core::Patcher::new(config)?;
    
    if let Some(patch_path) = manual_patch {
        tracing::info!("Applying manual patch: {}", patch_path);
        patcher.manual_patch(std::path::Path::new(&patch_path)).await?;
    } else {
        tracing::info!("Starting full patch process");
        patcher.run_full_patch().await?;
    }
    
    tracing::info!("Patching completed successfully");
    Ok(())
}

<div align="center">

# 🚀 Beam Patcher

### Modern Ragnarok Online Patcher System

A powerful, cross-platform game patcher for Ragnarok Online built with Rust and Tauri.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/tauri-%2324C8DB.svg?style=flat&logo=tauri&logoColor=%23FFFFFF)](https://tauri.app/)

[Features](#features) • [Installation](#installation) • [Documentation](#configuration) • [Contributing](#contributing) • [Support](#support)

---

</div>

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Features](#features)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [File Formats](#file-formats)
- [Development](#development)
- [API Reference](#api-reference)

## Overview

Beam Patcher is a complete game patching solution designed for Ragnarok Online servers. It supports multiple patch formats (GRF, BEAM), provides a modern web-based UI, and includes advanced features like mirror support, auto-updates, and parallel downloads.

### Key Highlights

- **Modern UI**: Web-based interface built with Tauri for native performance
- **Multi-Format**: Supports GRF (0x101, 0x102, 0x103, 0x200), and custom BEAM format
- **Parallel Downloads**: Multi-threaded downloading with mirror fallback
- **Customizable**: Fully configurable UI with custom layouts, buttons, news feeds
- **Cross-Platform**: Windows, Linux, and macOS support
- **Auto-Update**: Built-in self-updater for patcher maintenance

## Architecture

The project follows a modular workspace structure:

```
patchergame/
├── beam-core/           # Core patching logic and business rules
├── beam-formats/        # File format readers/writers (GRF, BEAM)
├── beam-patcher/        # Main executable and CLI
└── beam-ui/             # Tauri-based GUI application
```

### Component Overview

#### **beam-core**
Core library containing all business logic:
- **config**: Configuration management (YAML)
- **downloader**: HTTP download with resume support
- **parallel_downloader**: Multi-threaded download manager
- **patcher**: Patch application engine
- **verifier**: File integrity verification (MD5, SHA256)
- **updater**: Self-update mechanism
- **game_settings**: Game configuration management
- **server_checker**: Server status monitoring
- **client_checker**: Client validation

#### **beam-formats**
File format implementations:
- **grf**: GRF archive reader/writer (all versions)
- **rgz**: RGZ patch format (update soon)
- **gpf**: GPF patch format (update soon)
- **beam**: Custom BEAM format with MD5 verification

#### **beam-patcher**
Main executable that orchestrates the patching process. Integrates all components and provides CLI interface.

#### **beam-ui**
Tauri-based desktop application providing:
- Modern web-based interface
- Real-time progress tracking
- News feed and server status
- Custom layouts and branding
- Background video/audio support

## Features

### Patching Features
- ✅ Full GRF support (0x101, 0x102, 0x103, 0x200, (0x201 or 0x300 that is custom encryption grf for future update))
- ✅ Custom BEAM format with built-in MD5 verification
- ✅ Incremental patching
- ✅ Resume interrupted downloads
- ✅ Checksum verification (MD5, SHA256)
- ✅ Parallel downloads with multiple mirrors
- ✅ Automatic mirror fallback

### UI Features
- ✅ Responsive web-based interface
- ✅ Custom layouts and branding
- ✅ Background video/image support
- ✅ BGM audio playback
- ✅ News feed integration
- ✅ Server status display
- ✅ Custom buttons (website, forum, wiki, etc.)
- ✅ Real-time progress bars
- ✅ File-level download tracking

### Advanced Features
- ✅ Auto-updater for patcher
- ✅ Server connectivity check
- ✅ Client validation
- ✅ Game settings management
- ✅ Multi-language support (configurable)

## Installation

### Prerequisites
- Rust 1.75+ (for building from source)
- Windows 10+ / Linux / macOS

### From Release Binary

1. Download the latest release from your distribution server
2. Extract to desired location
3. Copy `config.example.yml` to `config.yml`
4. Configure your server settings (see [Configuration](#configuration))
5. Run `beam-patcher.exe` (Windows) or `beam-patcher` (Linux/macOS)

### Building from Source

```bash
# Clone repository
cd patchergame

# Build release version
cargo build --release

# Binaries will be in target/release/
# - beam-patcher.exe (main patcher)
```

### Release Folder Structure

When deploying the patcher, you need the following folder structure:

```
BeamPatcher/                    # Root release folder
├── beam-patcher.exe            # Main patcher executable
├── config.yml                  # Configuration file
├── assets/                     # Media assets folder
│   ├── logo.png               # Patcher logo (optional)
│   ├── background.jpg         # Background image (optional)
│   ├── bgm.mp3                # Background music (optional)
│   ├── trailer.mp4            # Background video (optional)
│   └── icons/                 # Custom button icons (optional)
│       ├── button-website.png
│       ├── button-forum.png
│       └── button-discord.png
└── data.grf or beam.grf       # Target GRF file (in game directory)
```

**Required Files:**
- `beam-patcher.exe` - Main executable
- `config.yml` - Configuration file

**Optional Folders:**
- `assets/` - Contains media files (logo, background, BGM, video)
- `assets/icons/` - Custom button icons

**Example Complete Setup:**
```
D:/YourRO/
├── beam-patcher.exe
├── config.yml
├── assets/
│   ├── logo.png              # 200x200px
│   ├── background.jpg        # 1920x1080px
│   ├── bgm.mp3              # Background music loop
│   ├── trailer.mp4          # 1920x1080px promo video
│   └── icons/
│       ├── button-website.png   # 200x60px
│       ├── button-forum.png     # 200x60px
│       └── button-discord.png   # 200x60px
├── data.grf                  # Game files
├── client.exe               # Game client
└── [other game files]
```

**Notes:**
- All paths in `config.yml` are relative to the patcher executable location
- If assets are not provided, the patcher will use default UI without media
- Video background requires `video_background_enabled: true` in config
- BGM requires `bgm_autoplay: true` in config
- Custom buttons require entries in `ui.custom_buttons` section

### Quick Start

```bash
# Run in debug mode
cargo run --bin beam-patcher

# Run in release mode
cargo run --release --bin beam-patcher

# Or use the debug batch script
run_patcher_debug.bat
```

## Configuration

### Main Configuration File: `config.yml`

```yaml
app:
  name: "Beam Patcher"
  version: "1.0.0"
  window_title: "Beam Patcher - Modern RO Patcher"
  game_directory: "D:\ro\game\YOUR RO"
  client_exe: "your client.exe"
  setup_exe: null
  bgm_autoplay: true
  bgm_file: "assets/your audio.mp3"
  server_name: "YOUR RO"
  video_background_enabled: true
  video_background_file: "assets/your video.mp4"

patcher:
  mirrors:
    - name: "Primary Mirror"
      url: "https://patch.yourserver.com"
      priority: 1
    - name: "Secondary Mirror"
      url: "https://patch2.yourserver.com"
      priority: 2
  
  patch_list_url: "https://patch.yourserver.com/patchlist.txt"
  target_grf: "data.grf"
  allow_manual_patch: true
  verify_checksums: true

ui:
  theme: "default"
  custom_css: null
  logo: null
  background: null
  show_progress: true
  show_file_list: true
  news_feed_url: "https://yourserver.com/api/news"
  server_status_url: "https://yourserver.com/api/status"
  custom_buttons:
    - label: "Website"
      url: "https://yourserver.com"
      icon: "assets/button-website.png"
      position: { x: "50px", y: "300px" }
  layout:
    width: 800
    height: 600
    use_custom_layout: false

updater:
  enabled: true
  check_url: "https://patch.yourserver.com/version.json"
  update_url: "https://patch.yourserver.com/updates"
  auto_update: false

server:
  login_server_ip: "127.0.0.1"
  login_server_port: 6900
  char_server_ip: "127.0.0.1"
  char_server_port: 6121
  map_server_ip: "127.0.0.1"
  map_server_port: 5121
```

### Configuration Sections

#### **app**
Application-level settings:
- `name`: Display name
- `version`: Patcher version
- `window_title`: Window title bar text
- `game_directory`: Game installation path
- `client_exe`: Game client executable name
- `setup_exe`: Setup/config executable (optional)
- `bgm_autoplay`: Auto-play background music
- `bgm_file`: BGM audio file path
- `server_name`: Server name display
- `video_background_enabled`: Enable video background
- `video_background_file`: Video file path

#### **patcher**
Patching behavior:
- `mirrors`: List of download mirrors (priority-ordered)
- `patch_list_url`: URL to patchlist.txt
- `target_grf`: Target GRF filename
- `allow_manual_patch`: Allow manual patch file selection
- `verify_checksums`: Verify file integrity

#### **ui**
UI customization:
- `theme`: UI theme name
- `custom_css`: Custom CSS file path
- `logo`: Logo image path
- `background`: Background image path
- `show_progress`: Show progress bars
- `show_file_list`: Show file download list
- `news_feed_url`: News API endpoint
- `server_status_url`: Server status API endpoint
- `custom_buttons`: Custom button definitions
- `layout`: Window dimensions and layout mode


#### **updater**
Auto-update configuration:
- `enabled`: Enable auto-update checks
- `check_url`: Version check endpoint
- `update_url`: Update download URL
- `auto_update`: Automatically download updates

#### **server**
Game server connection settings:
- `login_server_ip`: Login server IP
- `login_server_port`: Login server port
- `char_server_ip`: Character server IP
- `char_server_port`: Character server port
- `map_server_ip`: Map server IP
- `map_server_port`: Map server port

### Patch List Format: `patchlist.txt`

```
# Beam Patcher Patch List
# Format: filename [checksum]

# BEAM patches (recommended)
patch_v1.0.1.beam f5e6d7c8b9a0123456789abcdef0123456789abcdef0123456789abcdef012345
```

**Format**: Each line contains:
- Filename (required)
- Checksum (SHA256 hex, optional but recommended)
- Lines starting with `#` are comments

### Version Info Format: `version.json`

```json
{
  "version": "1.0.0",
  "download_url": "https://patch.yourserver.com/updates/beam-patcher-v1.0.0.exe",
  "changelog": "Initial release\n\n- Feature 1\n- Feature 2",
  "required": false
}
```

### News Feed API Format

```json
[
  {
    "title": "New Year Event",
    "date": "2024-01-01",
    "category": "EVENT",
    "content": "Event details..."
  },
  {
    "title": "Maintenance Notice",
    "date": "2024-01-05",
    "category": "MAINTENANCE",
    "content": "Maintenance info..."
  }
]
```

### Server Status API Format

```json
{
  "online": true,
  "players": 1234,
  "uptime": "5 days 3 hours",
  "status": "Online"
}
```

## Usage

### Running the Patcher

#### Windows
```cmd
beam-patcher.exe
```

#### Linux/macOS
```bash
./beam-patcher
```

### Command-Line Options

```bash
beam-patcher [OPTIONS]

OPTIONS:
  -c, --config <FILE>    Use custom config file [default: config.yml]
  -v, --verbose         Enable verbose logging
  -h, --help            Print help information
  -V, --version         Print version information
```

### Patching Process Flow

1. **Initialization**
   - Load configuration
   - Initialize UI
   - Check for patcher updates (if enabled)

2. **Pre-Patch Checks**
   - Verify game directory exists
   - Check server connectivity (optional)
   - Validate client executable (optional)

3. **Patch Discovery**
   - Download patchlist.txt
   - Parse patch entries
   - Calculate required downloads

4. **Download Phase**
   - Download patches from mirrors (parallel)
   - Verify checksums
   - Resume interrupted downloads
   - Fallback to alternate mirrors on failure

5. **Patch Application**
   - Extract patch contents
   - Verify file integrity
   - Apply to target GRF
   - Update GRF file table

6. **Post-Patch**
   - Verify final state
   - Clean up temporary files
   - Launch game (optional)

## File Formats

### GRF Format

GRF (Game Resource File) is the primary archive format for Ragnarok Online.

**Supported Versions:**
- 0x101 (Ancient)
- 0x102 (Early)
- 0x103 (Common)
- 0x200 (Modern)
- 0x201 (custom encryption for future coz need reverse engineering)
- 0x300 (custom encryption for future coz need reverse engineering)

```

**Advantages:**
- ✅ MD5 verification per file
- ✅ Zlib compression
- ✅ Simple, fast parsing
- ✅ Cross-platform compatibility
- ✅ Built-in corruption detection

## Development

### Project Structure

```
patchergame/
├── beam-core/                 # Core library
│   ├── src/
│   │   ├── config.rs         # Configuration management
│   │   ├── downloader.rs     # HTTP downloader
│   │   ├── parallel_downloader.rs  # Multi-threaded downloader
│   │   ├── patcher.rs        # Patch engine
│   │   ├── verifier.rs       # Checksum verification
│   │   ├── updater.rs        # Auto-updater
│   │   ├── sso.rs            # SSO client
│   │   ├── game_settings.rs  # Game config
│   │   ├── server_checker.rs # Server status
│   │   ├── client_checker.rs # Client validation
│   │   ├── error.rs          # Error types
│   │   └── lib.rs
│   └── Cargo.toml
│
├── beam-formats/              # Format implementations
│   ├── src/
│   │   ├── grf.rs            # GRF reader/writer
│   │   ├── beam.rs           # BEAM format
│   │   ├── error.rs          # Format errors
│   │   └── lib.rs
│   │   
│   └── Cargo.toml
│
├── beam-patcher/              # Main executable
│   ├── src/
│   │   └── main.rs           # Entry point & CLI
│   └── Cargo.toml
│
├── beam-ui/                   # GUI application
│   ├── src/
│   │   └── main.rs           # Tauri app
│   ├── public/                # Web assets
│   │   ├── index.html
│   │   └── assets/
│   ├── tauri.conf.json       # Tauri config
│   └── Cargo.toml
│
├── examples/                  # Example files
│   ├── news.json
│   ├── status.json
│   └── README.txt
│
├── Cargo.toml                 # Workspace config
├── Cargo.lock
├── config.example.yml         # Example configuration
├── config.production.yml      # Production example
├── patchlist.example.txt      # Example patchlist
├── version.example.json       # Example version info
└── README.md                  # This file
```

### Building

```bash
# Build all components
cargo build --release

# Build specific component
cargo build --release -p beam-patcher

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run --bin beam-patcher

# Build optimized
cargo build --release --features "optimized"
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_grf_format

# Run with output
cargo test -- --nocapture

# Test specific package
cargo test -p beam-formats
```

### Debugging

```bash
# Enable debug logging
set RUST_LOG=debug
beam-patcher.exe

# Or use the debug batch script
run_patcher_debug.bat

# Check config
type config.yml

# Verify patchlist
curl https://patch.yourserver.com/patchlist.txt
```

## API Reference

### beam-core API

#### Config

```rust
use beam_core::config::Config;

// Load config
let config = Config::from_file("config.yml")?;

// Access settings
let mirrors = &config.patcher.mirrors;
let target_grf = &config.patcher.target_grf;
```

#### Downloader

```rust
use beam_core::downloader::Downloader;

let downloader = Downloader::new(mirrors);
let patch_data = downloader.download_patch("patch.beam").await?;
```

#### Parallel Downloader

```rust
use beam_core::parallel_downloader::ParallelDownloader;

let downloader = ParallelDownloader::new(mirrors, 4); // 4 threads
let patches = downloader.download_all(patch_list).await?;
```

#### Patcher

```rust
use beam_core::patcher::Patcher;

let patcher = Patcher::new(&config);
patcher.apply_patch("patch.beam").await?;
```

#### Verifier

```rust
use beam_core::verifier::Verifier;

let verifier = Verifier::new();
let is_valid = verifier.verify_file("file.dat", expected_hash)?;
```

#### Updater

```rust
use beam_core::updater::Updater;

let updater = Updater::new(&config.updater);
if let Some(new_version) = updater.check_update().await? {
    updater.download_update(&new_version).await?;
}
```

### beam-formats API

#### GRF

```rust
use beam_formats::grf::Grf;

// Open existing GRF
let mut grf = Grf::open("data.grf")?;

// Add file
grf.add_file("data/texture.bmp", &file_data)?;

// Extract file
let data = grf.get_file("data/texture.bmp")?;

// Save changes
grf.save()?;
```

#### BEAM

```rust
use beam_formats::beam::BeamArchive;

// Create archive
let mut beam = BeamArchive::new();
beam.add_file("data/file.txt", &data)?;
beam.write("patch.beam")?;

// Read archive
let beam = BeamArchive::read("patch.beam")?;
let data = beam.get_file("data/file.txt")?;

// Verify integrity
beam.verify_all()?;
```

## Disclaimer

This patcher is designed for legitimate use with Ragnarok Online private servers. It supports multiple GRF formats including standard formats (0x101-0x200) and custom encryption formats (0x300/Gepard Shield) later for future updates.

**Intended Use:**
- ✅ Private server owners managing game updates
- ✅ Server administrators with proper authorization
- ✅ Developers creating custom game clients
- ✅ Educational purposes (understanding GRF format)

**Important:** This tool should only be used with proper authorization from server owners. Unauthorized use may violate terms of service.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**Ways to contribute:**
- 🐛 Report bugs via GitHub Issues
- 💡 Suggest features via GitHub Discussions
- 🎨 Create and share custom themes
- 📝 Improve documentation
- 🔧 Submit pull requests for bug fixes or enhancements

## License

Dual-licensed under:
- **MIT License** ([LICENSE-MIT](LICENSE-MIT))
- **Apache License 2.0** ([LICENSE-APACHE](LICENSE-APACHE))

You may choose either license for your use.

## Authors & Credits

**Created by:** [@beamguide](https://github.com/beamguide)  
**Discord:** beamguide#9797

This project is open source and maintained by the Ragnarok Online private server community.

### Acknowledgments

- Built with [Rust](https://www.rust-lang.org/) and [Tauri](https://tauri.app/)
- GRF format documentation from the RO community
- Inspired by other RO patchers (rpatchur, thor patcher)

## Support

**For Users:**
- 📖 Check the [documentation](docs/) for setup guides
- 💬 Join [GitHub Discussions](../../discussions) for Q&A
- 🐛 Report issues on [GitHub Issues](../../issues)

**For Server Owners:**
- See [Configuration Guide](docs/CONFIGURATION.md) for server setup
- See [Theme Guide](docs/THEMES.md) for custom branding

**For Developers:**
- See [Building Guide](docs/BUILDING.md) for development setup
- See [Contributing Guide](CONTRIBUTING.md) for contribution guidelines

---

**⭐ If you find this project useful, please consider giving it a star!**

Made with ❤️ for the Ragnarok Online community

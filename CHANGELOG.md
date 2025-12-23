# Changelog

All notable changes to Beam Patcher will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Initial Open Source Release

## [1.0.0] - 2024-12-23

### Added
- 🎉 **Initial Release** - Beam Patcher is now open source!
- ✅ Full GRF support (versions 0x101, 0x102, 0x103, 0x200, 0x300)
- ✅ Modern Tauri-based web UI
- ✅ Multiple patch format support (RGZ, GPF, BEAM)
- ✅ Custom BEAM format with MD5 verification
- ✅ Parallel downloads with mirror support
- ✅ Auto-updater functionality
- ✅ SSO/OAuth2 authentication support
- ✅ Customizable themes and layouts
- ✅ Background video and audio support
- ✅ Server status checking
- ✅ Client validation
- ✅ News feed integration
- ✅ Custom button system
- ✅ Game settings management
- ✅ Cross-platform support (Windows, Linux, macOS)

### Core Features
- **beam-core**: Core patching engine with parallel downloads
- **beam-formats**: GRF, RGZ, GPF, and BEAM format implementations
- **beam-patcher**: Main executable with CLI interface
- **beam-ui**: Modern Tauri-based GUI

### Documentation
- 📖 Comprehensive README with setup guides
- 📖 Configuration documentation
- 📖 Theme customization guide
- 📖 API reference
- 📖 Contributing guidelines

### Security
- ✅ Checksum verification (MD5, SHA256)
- ✅ File integrity validation
- ✅ Secure download with resume support
- ✅ OAuth2 authentication ready

### Performance
- ⚡ Multi-threaded downloads
- ⚡ Efficient GRF patching
- ⚡ Zlib compression support
- ⚡ Memory-optimized file handling

## Future Plans

### Planned Features
- [ ] Multiple language support (i18n)
- [ ] Plugin system for extensibility
- [ ] Advanced logging and debugging
- [ ] Patch creation wizard
- [ ] Rollback/restore functionality
- [ ] Delta patching for efficiency
- [ ] Torrent support for large patches
- [ ] Discord Rich Presence integration
- [ ] Automatic backup before patching
- [ ] Bandwidth limiting options

### Community Requests
- [ ] Theme marketplace
- [ ] Web-based patch manager
- [ ] Mobile companion app
- [ ] Patch analytics dashboard

## Version History

### Version Numbering
- **Major.Minor.Patch** (e.g., 1.0.0)
- **Major**: Breaking changes or major features
- **Minor**: New features, backwards compatible
- **Patch**: Bug fixes and minor improvements

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on contributing to this changelog.

### How to Report Changes
When contributing, please update this file following the format:
```markdown
### Added
- New feature description

### Changed
- Modified feature description

### Fixed
- Bug fix description

### Removed
- Removed feature description
```

---

**Legend:**
- 🎉 Major milestone
- ✅ New feature
- 🐛 Bug fix
- 📖 Documentation
- ⚡ Performance
- 🔒 Security
- 💥 Breaking change
- 🗑️ Deprecated

**Note:** This project was initially developed privately and is now open sourced for the Ragnarok Online community.

**Creator:** [@beamguide](https://github.com/beamguide)  
**Discord:** beamguide#9797

# Beam Patcher Themes

This directory contains community-contributed themes for Beam Patcher.

## Using a Theme

1. Choose a theme from the folders below
2. Copy the theme's `config.yml` to your patcher root directory
3. Copy the theme's `assets/` folder to your patcher root directory
4. Run the patcher

## Available Themes

### Default
Basic theme with minimal customization. Great starting point for creating your own theme.

### Dark Mode (Coming Soon)
A sleek dark theme for night patching sessions.

### Classic RO (Coming Soon)
Nostalgic theme inspired by classic Ragnarok Online aesthetics.

## Creating Your Own Theme

### Quick Start

1. Create a new folder in `themes/` with your theme name
2. Create the following structure:
   ```
   themes/your-theme/
   ├── README.md          # Theme description
   ├── config.yml         # Theme configuration
   └── assets/            # Theme assets
       ├── logo.png
       ├── background.jpg
       ├── bgm.mp3 (optional)
       ├── trailer.mp4 (optional)
       └── icons/
           ├── button-website.png
           ├── button-forum.png
           └── button-discord.png
   ```

### Configuration Template

```yaml
app:
  name: "Your Server Name"
  window_title: "Your Patcher Title"
  bgm_file: "assets/bgm.mp3"
  video_background_file: "assets/trailer.mp4"

ui:
  logo: "assets/logo.png"
  background: "assets/background.jpg"
  custom_buttons:
    - label: "Website"
      url: "https://yourserver.com"
      icon: "assets/icons/button-website.png"
      position: { x: "50px", y: "300px" }
```

### Asset Guidelines

#### Logo (`logo.png`)
- **Recommended size**: 200x200px
- **Format**: PNG with transparency
- **Max size**: 500KB

#### Background (`background.jpg`)
- **Recommended size**: 1920x1080px (16:9)
- **Format**: JPG or PNG
- **Max size**: 2MB

#### Video Background (`trailer.mp4`)
- **Recommended size**: 1920x1080px (16:9)
- **Format**: MP4 (H.264)
- **Duration**: 30-60 seconds (loops)
- **Max size**: 20MB

#### Background Music (`bgm.mp3`)
- **Format**: MP3
- **Duration**: 2-5 minutes (loops)
- **Bitrate**: 128-192 kbps
- **Max size**: 5MB

#### Button Icons (`icons/*.png`)
- **Recommended size**: 200x60px
- **Format**: PNG with transparency
- **Max size**: 100KB each

### Color Customization

You can customize colors in `config.yml`:

```yaml
ui:
  colors:
    primary: "#ff6600"      # Main accent color
    secondary: "#333333"    # Secondary elements
    text: "#ffffff"         # Text color
    background: "#1a1a1a"   # UI background
    progress: "#00ff00"     # Progress bar
```

### Testing Your Theme

1. Copy your theme files to the patcher directory
2. Run: `beam-patcher.exe`
3. Check all elements display correctly
4. Test on different screen resolutions
5. Verify all buttons work
6. Check video/audio playback (if used)

## Submitting Your Theme

1. Fork the repository
2. Add your theme to `themes/`
3. Include a `README.md` with:
   - Theme name and description
   - Screenshots (at least 2)
   - Installation instructions
   - Credits for assets used
4. Submit a pull request with label `theme`

### Theme Submission Checklist

- [ ] Theme folder created in `themes/`
- [ ] README.md with description and screenshots
- [ ] All assets included and optimized
- [ ] config.yml configured correctly
- [ ] Tested on at least one platform
- [ ] No copyrighted content without permission
- [ ] License specified (if different from project)

## Theme Showcase

<!-- Add your theme here after approval -->

## License & Copyright

- Themes must have clear licensing
- Don't use copyrighted content without permission
- Credit original artists/sources
- Recommended licenses: MIT, CC BY 4.0, or "Free for RO servers"

## Resources

**Free Assets:**
- [OpenGameArt](https://opengameart.org/)
- [Kenney Assets](https://www.kenney.nl/assets)
- [Freesound](https://freesound.org/)
- [Pixabay](https://pixabay.com/)

**Design Tools:**
- [GIMP](https://www.gimp.org/) - Free image editor
- [Inkscape](https://inkscape.org/) - Free vector graphics
- [Audacity](https://www.audacityteam.org/) - Free audio editor
- [Figma](https://www.figma.com/) - Design mockups

## Support

Need help creating a theme? Ask in:
- GitHub Discussions
- Discord: beamguide#9797

---

**Happy theming!** 🎨

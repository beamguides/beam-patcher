Icon Files untuk Beam Patcher
================================

Struktur folder ini:
beam-ui/icons/
├── 32x32.png       - Icon 32x32 pixels (Windows taskbar)
├── 128x128.png     - Icon 128x128 pixels (Windows installer)
├── 128x128@2x.png  - Icon 256x256 pixels (High DPI)
├── icon.ico        - Windows icon file (multi-resolution)
└── icon.icns       - macOS icon file (multi-resolution)

Cara membuat icons:

1. MANUAL - Buat file PNG dengan ukuran yang sesuai
   - 32x32.png   : 32 x 32 pixels
   - 128x128.png : 128 x 128 pixels
   - 128x128@2x.png : 256 x 256 pixels

2. MENGGUNAKAN TOOL ONLINE:
   - https://www.favicon-generator.org/
   - https://convertio.co/png-ico/
   - Upload image Anda (minimal 256x256px)
   - Download icon.ico untuk Windows

3. MENGGUNAKAN TAURI CLI (RECOMMENDED):
   - Buat satu file PNG besar (512x512 atau 1024x1024)
   - Simpan sebagai "app-icon.png"
   - Run: cargo tauri icon app-icon.png
   - Semua icon akan di-generate otomatis

4. ALTERNATIF - Gunakan default placeholder:
   - Tauri akan generate default icon jika tidak ada
   - Tapi lebih baik pakai icon custom

Format yang dibutuhkan:
- PNG: Transparent background, square (1:1 ratio)
- ICO: Multi-resolution (16, 32, 48, 64, 128, 256)
- ICNS: Multi-resolution untuk macOS

Tips:
- Gunakan vector graphics (SVG) sebagai source
- Export ke PNG dengan resolusi tinggi dulu (1024x1024)
- Lalu resize ke ukuran yang dibutuhkan
- Pastikan terlihat jelas di size kecil (32x32)

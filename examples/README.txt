EXAMPLE FILES FOR BEAM PATCHER CUSTOM LAYOUT
=============================================

Folder ini berisi contoh file untuk custom layout patcher.

FILES:
------
1. news.json       - Contoh format news feed API
2. status.json     - Contoh format server status API
3. README.txt      - File ini

CARA PAKAI:
-----------
1. Copy config.custom-layout.example.yml ke config.yml
2. Buat folder "assets" di folder patcher
3. Taruh background image dan button images di folder assets
4. Setup web server untuk serve news.json dan status.json
   Atau gunakan file lokal dengan file:// protocol

CONTOH STRUKTUR FOLDER:
-----------------------
patchergame/
├── beam-patcher.exe
├── config.yml
├── assets/
│   ├── background.jpg          (900x700px)
│   ├── button-website.png      (200x60px)
│   ├── button-forum.png        (200x60px)
│   ├── button-kb.png           (200x60px)
│   └── start-button.png        (150x50px)
└── examples/
    ├── news.json
    └── status.json

API ENDPOINTS:
--------------
Untuk production, buat API endpoints yang return JSON:

News API: GET /api/news
Response:
[
  {
    "title": "Event Title",
    "date": "22 Dec, 2024",
    "category": "EVENT"
  }
]

Status API: GET /api/status  
Response:
{
  "online": true,
  "players": 1234
}

BUTTON IMAGES:
--------------
Untuk hasil terbaik:
- Width: 180-220px
- Format: PNG dengan transparency
- Gunakan tools seperti Photoshop/GIMP
- Buat design yang sesuai theme server

BACKGROUND IMAGE:
-----------------
- Size: 900x700px (atau sesuai layout config)
- Format: JPG (untuk file size lebih kecil) atau PNG
- Pastikan progress bar dan text masih terlihat jelas

POSITIONING:
------------
Posisi dalam pixels dari top-left corner:
- Buttons kiri: x: "50px", y: "300px", "370px", "440px", dst
- News panel: default di tengah/kanan atas
- Progress bar: default di bawah tengah
- START button: default di kanan bawah

Customize posisi via CSS jika perlu lebih detail.

TROUBLESHOOTING:
----------------
- Images tidak muncul? Check path di config.yml
- News tidak load? Check API endpoint bisa diakses
- Layout berantakan? Set use_custom_layout: false
- Button tidak clickable? Check z-index di CSS

SUPPORT:
--------
Lihat CUSTOM_LAYOUT_GUIDE.md untuk dokumentasi lengkap.

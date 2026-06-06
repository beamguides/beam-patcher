# Tauri 1.x → 2.x migration notes

> **Status:** not started. Tauri 1.x (1.5/1.8) is still maintained for now,
> but 2.x is the supported track going forward. Migration is mechanical but
> touches many files; budget half a day.

## Tooling steps

```powershell
# 1. Install the v2 CLI globally (or as a dev-dep)
cargo install tauri-cli --version "^2" --locked

# 2. Run the official migrator from the workspace root
cd D:\bg.dev\beam-patcher
cargo tauri migrate
```

`cargo tauri migrate` will:
- bump `tauri` and `tauri-build` to `2.x` in every `Cargo.toml`
- rewrite `tauri.conf.json` (schema, allow-list → permissions, bundle layout)
- generate a `capabilities/` directory replacing the old allow-list
- update Node / pnpm side if you have a `package.json` frontend

You should still hand-verify the items below.

## Code-level changes required in this repo

The grep audit below was run on `beam-ui/src/`. Each entry is something the
migrator may not fix automatically, or that you'll want to double-check.

### `beam-ui/src/lib.rs`
- `tauri::Builder::default()` → unchanged.
- `.invoke_handler(tauri::generate_handler![…])` → unchanged.
- `.run(tauri::generate_context!("tauri.conf.json"))` → unchanged but
  `tauri.conf.json` itself is rewritten — review the diff.

### `beam-ui/src/commands.rs`

| v1 API | v2 replacement |
|---|---|
| `use tauri::{State, AppHandle, Manager};` | Add `tauri::Emitter` (the `emit_all` method moved). |
| `app.emit_all("patch-progress", snapshot)` | `app.emit("patch-progress", snapshot)` — `emit` replaces `emit_all`. Single-window emit becomes `app.emit_to("window-label", "event", payload)`. |
| `tauri::api::dialog::blocking::FileDialogBuilder::new()` | Move to plugin: add `tauri-plugin-dialog` to `Cargo.toml`, register with `.plugin(tauri_plugin_dialog::init())`, then `use tauri_plugin_dialog::DialogExt;` and `app.dialog().file().blocking_pick_folder()`. |
| `tauri::api::dialog::blocking::FileDialogBuilder::new().pick_file()` | `app.dialog().file().blocking_pick_file()` from the same plugin. |
| `app.path_resolver().resolve_resource(&path)` | `app.path().resolve(&path, BaseDirectory::Resource)` from `tauri::Manager` + `tauri::path::BaseDirectory`. |

### `tauri.conf.json` (will be rewritten by migrator)
- `tauri.allowlist` → split into a `capabilities/` directory of JSON files,
  each scoped to a window label. The old `"dialog-open"`, `"http-all"`,
  `"shell-open"` flags become explicit permissions per plugin.
- `tauri.updater` config moves to the `tauri-plugin-updater` plugin block.
- `tauri.bundle.identifier` is now mandatory in reverse-DNS form
  (e.g. `id.beamguides.patcher`).

### `Cargo.toml` (beam-ui)
- Bump `tauri = "1.5"` → `tauri = "2"`, `tauri-build = "1.5"` → `tauri-build = "2"`.
- Drop the old feature flags (`dialog-open`, `http-all`, `shell-open`, etc.) —
  they no longer exist in v2.
- Add the plugin crates you actually use:
  ```toml
  tauri-plugin-dialog  = "2"
  tauri-plugin-shell   = "2"
  tauri-plugin-updater = "2"
  ```

### Updater plugin
The standalone `self_update` crate in `beam-core/updater.rs` is independent
of Tauri and can stay. But if you'd rather use Tauri's signed-update mechanism,
swap to `tauri-plugin-updater` and drop `self_update`.

## Estimated work

- Auto migrator: ~5 min.
- Manual diff review (conf.json, capabilities, allow-list mapping): ~30 min.
- Code fixes in `commands.rs` (4 call sites): ~15 min.
- `cargo check`, fix fallout: ~20 min.
- Smoke test (window opens, patching, dialog open, news fetch, launch): ~30 min.

Total: **~2 hours** for a single experienced operator.

## Recommendation

Defer until either:
1. You hit a v1 bug fixed only in v2, or
2. You want to ship the `signed-updater` feature.

Both can wait — current v1 build is healthy.

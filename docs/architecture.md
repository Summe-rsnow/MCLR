# MCLR Architecture

## Stack
- **Shell**: Tauri 2.x stable (`tauri = "2"`, `@tauri-apps/cli@^2`), WebView2 on Windows 10/11 x64.
- **Backend**: Rust edition 2024, `rust-version = "1.99"`, Cargo workspace (`resolver = "3"`), `tokio`, `reqwest` (rustls).
- **Frontend**: Vue 3.5 + TypeScript + Vite 8 + Pinia + vue-router (hash history) + vue-i18n 11 (composition mode) + UnoCSS (`presetWind4`, `presetIcons` with `@iconify-json/lucide`) + `culori` (OKLCH palette).
- **IPC Typing**: `tauri-specta` + `specta` generating `src/ipc/bindings.ts`.

## Crate Graph
- `mclr-core`: paths, settings store, `AppError`, logging, `TaskCenter`, secrets, events, shared traits (`AccountSession`).
- `mclr-net`: HTTP client, proxy/DoH, mirror rewrite, source racing, chunked downloader.
- `mclr-game`: version JSON model, rules, libraries, assets, instances, Java manager, launch pipeline, process watch.
- `mclr-auth`: offline, Microsoft, authlib/Yggdrasil, Yggdrasil Connect, skin/cape, DPAPI+AES-256-GCM account store.
- `mclr-loaders`: Fabric, LegacyFabric, Forge/NeoForge/Cleanroom, OptiFine, LiteLoader, LabyMod installers.
- `mclr-comp`: Modrinth, CurseForge, unified model, dependency resolver, local mod scan, JarInJar, modpack import/export.
- `mclr-saves`: NBT (`fastnbt`), saves, datapacks, `servers.dat`, SLP ping/SRV.
- `mclr-crash`: crash analysis (45 causes) + report export.
- `mclr-update`: self-update wrapper around `tauri-plugin-updater`.
- `mclr-cli`: headless smoke CLI (`clap`).
- `mclr-app` (`src-tauri`): Tauri shell, window vibrancy, command registry.

## Async & Non-Blocking Runtime (`src-tauri/src/`)
- **`ipc::run_blocking`**: wraps `tokio::task::spawn_blocking` and maps `JoinError` to `AppError("app.blockingJoin")`; mandatory for synchronous filesystem/archive/hash/NBT/image work in `#[tauri::command]` handlers.
- **`http::HttpState`**: managed `RwLock<(NetSettings, reqwest::Client)>` state shared across all IPC commands and rebuilt only when `settings.net` changes on `settings_patch`.
- **`events::Batcher<T>`**: coalesces high-frequency backend events into 50 ms / 64-item windows (`tasks://update` deduplicated by task ID, `launch://logBatch`, `resources://patch`).
- **`mclr_game::gamedir::resolve_game_dir`**: single source of truth for per-instance game directory isolation (`"on" | "off" | "global"` + `DefaultIsolation`), persisting the first computed `"global"` decision into `mclr.instance.json`.
- **Asset protocol**: enabled with `data_dir` and all configured `game.folders` allowed at startup plus `assets_allow` on user-selected media/folder paths so thumbnails, world icons, and pack icons load via `convertFileSrc` without base64 IPC payloads.

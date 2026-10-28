# MCLR Architecture

## Stack
- **Shell**: Tauri 2.x stable (`tauri = "2"`, `@tauri-apps/cli@^2`), WebView2 on Windows 10/11 x64.
- **Backend**: Rust edition 2024, `rust-version = "1.99"`, Cargo workspace (`resolver = "3"`), `tokio`, `reqwest` (rustls).
- **Frontend**: Vue 3.5 + TypeScript + Vite 8 + Pinia + vue-router (hash history) + vue-i18n 11 (composition mode) + UnoCSS (`presetWind4`, `presetIcons` with `@iconify-json/lucide`) + `culori` (OKLCH palette).
- **Toolchain**: Oxc suite (`oxlint` + `oxfmt`) for lightning-fast linting and code formatting; `tauri-specta` + `specta` generating `src/ipc/bindings.ts`.

---

## Crate Graph

- `mclr-core`: paths, settings store, `AppError`, daily rolling logging, `TaskCenter`, secrets, events, shared traits (`AccountSession`).
- `mclr-net`: HTTP client, proxy/DoH (`hickory-resolver`), mirror rewrite, source racing ladder, chunked downloader with dynamic work-stealing.
- `mclr-game`: version JSON model, rules, libraries, assets, instances, Java manager, launch pipeline, process watch, install pipeline.
- `mclr-auth`: offline, Microsoft OAuth 2.0 (device code flow), authlib-injector/Yggdrasil, Yggdrasil Connect, skin/cape, DPAPI+AES-256-GCM encrypted account store.
- `mclr-loaders`: Fabric, LegacyFabric, Forge, NeoForge, Cleanroom, OptiFine, LiteLoader, LabyMod 4 installers and compatibility resolution.
- `mclr-comp`: Modrinth, CurseForge, unified metadata model, dependency resolver, local mod scan, Jar-in-Jar inspector, modpack import/export (`.mrpack`, CurseForge `.zip`).
- `mclr-saves`: NBT (`fastnbt`), saves, datapacks, `servers.dat`, SLP ping/SRV (Happy Eyeballs RFC 8305).
- `mclr-crash`: crash analysis (heuristic classifier covering 45 failure causes) + diagnostic `.zip` report export.
- `mclr-update`: self-update wrapper around `tauri-plugin-updater`.
- `mclr-cli`: headless smoke CLI (`clap`).
- `mclr-app` (`src-tauri`): Tauri shell, window vibrancy/mica, command registry.

---

## Backend Submodule Decomposition

To ensure long-term maintainability, backend crates avoid oversized single-file implementations (>800 lines) by structuring domains into dedicated submodules:

- **`mclr-game::launch`**:
  - `launch/ram.rs`: Windows physical memory detection via `GlobalMemoryStatusEx` and auto-allocation calculations.
  - `launch/args.rs`: Command-line flag assembly, JVM argument parser, and `@argfile` generation.
  - `launch/win32.rs`: Win32 process creation flags (`CREATE_NO_WINDOW`, process job objects).
  - `launch/process.rs`: Asynchronous process monitoring, exit code tracking, and stdout/stderr stream piping.
  - `java.rs`: Java discovery/selection. Java runtimes are never downloaded by MCLR — the launcher only picks among installed ones. **There is no global "default Java"**: `javaSettings` carries only user-added paths (`custom`) and the JDK preference, and every launch derives the runtime from the version JSON's `javaVersion` (Java 21 for 1.21.x, Java 25 for 26.x). Ranking prefers an exact major match, then the closest major above it; anything below the requirement is never chosen. A per-instance pin is honoured only when it can run that version — otherwise a compatible install is used, or launch fails fast with `launch.java.tooOld` / `launch.java.notFound` and a one-click jump to Java management.
- **`mclr-game::instance`**:
  - `instance/classifier.rs`: Instance categorization (PCL-parity release/snapshot/ancient/fool classification).
  - `instance/scanner.rs`: Multi-directory instance scanning and version JSON parsing.
  - `instance/ops.rs`: Rename, star, delete, config mutation, and isolation directory operations.
- **`mclr-game::install`**:
  - `install/manifest.rs`: Official Mojang manifest and UVMC unlisted manifest fetching and merging.
  - `install/specs.rs`: Version specs, asset indices, and client jar download specifications.
  - `install/pipeline.rs`: Multi-stage task execution (assets, libraries, client, loader chains).
- **`mclr-saves::saves`**:
  - `saves/level_nbt.rs`: World `level.dat` parsing and mutation across all DataVersion eras.
  - `saves/archive.rs`: Zip backup creation and world folder restore pipeline.
  - `saves/screenshots.rs`: Screenshot metadata extraction, progressive disk thumbnails, and clipboard copy.
- **`mclr-comp::modpack`**:
  - `modpack/formats.rs`: Modpack archive format parsers (Modrinth `.mrpack`, CurseForge manifest, MultiMC, HMCL).
  - `modpack/installer.rs`: Modpack extraction and dependency download task orchestration.
- **`src-tauri::ipc::resources`**:
  - Extracted handlers for resource packs, shader packs, and world schematics to keep `localmods.rs` focused strictly on mod lifecycle.

---

## Modular IPC Client Architecture (`src/ipc/`)

Frontend IPC calls are strictly typed and modularized to prevent monolithic coupling:

```text
src/ipc/
├── bindings.ts          # Auto-generated Specta types & raw invoke wrappers
├── client.ts            # Unified facade re-exporting all domains for app-wide use
├── mock.ts              # Browser mock provider for standalone frontend dev
└── domains/             # Domain-specific client modules
    ├── accounts.ts      # Microsoft / Offline / Authlib account actions
    ├── comp.ts          # Modrinth / CurseForge community search & downloads
    ├── crash.ts         # Crash analysis and report generation
    ├── home.ts          # News feed and remote announcements
    ├── install.ts       # Version installation, manifest fetching & compat offload
    ├── instances.ts     # Instance scanning, creation, editing & isolation
    ├── launch.ts        # Game launch pipeline & process kill
    ├── media.ts         # Background media, asset paths & clipboard
    ├── resources.ts     # Mods, resource packs, shader packs, schematics
    ├── saves.ts         # World saves, datapacks, backups & screenshots
    ├── servers.ts       # Server list, SLP ping & quick-play
    ├── settings.ts      # Settings store get & JSON merge-patch
    ├── tasks.ts         # Task list, cancel, pause and resume
    ├── tools.ts         # Cache cleaning, desktop shortcuts, skin tools
    └── update.ts        # Self-updater checks & launcher logs
```

- **Frontend-to-Rust Offload (`installVersionCompat`)**: Complex version metadata validation (such as April Fools lore detection, loader incompatibility rules, and recommended Java runtime selection) is offloaded to pure Rust routines exposed via `ipcClient.installVersionCompat`.

---

## Global Overlay & Drawer Architecture

All floating, modal, and drawer surfaces follow a unified overlay architecture rooted in `MOverlay.vue`:

```text
       ┌────────────────────────┐
       │      MOverlay.vue      │ Teleports to <body>
       │  (Backdrop + Esc + Z)  │ Manages blur & motion
       └───────────┬────────────┘
                   │
         ┌─────────┴─────────┐
         ▼                   ▼
  ┌─────────────┐     ┌─────────────┐
  │ MDialog.vue │     │ MDrawer.vue │
  │   (Modal)   │     │   (Slide)   │
  └─────────────┘     └─────────────┘
```

- **`MOverlay.vue`**:
  - Teleports content to `<body>` and maintains a consistent backdrop layer (`bg-black/45` with `backdrop-blur-xs`).
  - Implements global `Escape` key trapping on the topmost active overlay.
  - Dynamically disables root page scrolling while active.
  - Responds to `html[data-motion='full' | 'reduced' | 'off']` by collapsing multi-axis animations to pure opacity or instantaneous transitions.
- **`MDialog.vue`**: Centered modal container with built-in header, action footer, confirm variant binding, and loading state.
- **`MDrawer.vue`**: Slide-out drawer (`placement="right" | "left" | "bottom"`) with standard widths (`sm` = 360px, `md` = 448px, `lg` = 560px, `xl` = 680px) and smooth hardware-accelerated drawer transitions.

---

## Async & Non-Blocking Runtime (`src-tauri/src/`)

- **`ipc::run_blocking`**: Wraps `tokio::task::spawn_blocking` and maps `JoinError` to `AppError("app.blockingJoin")`. **Mandatory** for synchronous filesystem, archive, hash, NBT, or image work in `#[tauri::command]` handlers to guarantee WebView2 IPC message loop responsiveness.
- **`http::HttpState`**: Managed `RwLock<(NetSettings, reqwest::Client)>` state shared across all IPC commands and rebuilt only when `settings.net` changes on `settings_patch`.
- **`events::Batcher<T>`**: Coalesces high-frequency backend events into 50 ms / 64-item windows (`tasks://update` deduplicated by task ID, `launch://logBatch`, `resources://patch`).
- **`mclr_game::gamedir::resolve_game_dir`**: Single source of truth for per-instance game directory isolation (`"on" | "off" | "global"` + `DefaultIsolation`), persisting the first computed `"global"` decision into `mclr.instance.json`.
- **Asset protocol**: Enabled with `data_dir` and all configured `game.folders` allowed at startup plus `assets_allow` on user-selected media/folder paths so thumbnails, world icons, and pack icons load via `convertFileSrc` without base64 IPC payloads.

---

## Daily Rolling Logging (`mclr-core::logging`)

- **Daily Log Suffix**: Configured via `tracing_appender::rolling::RollingFileAppender` with `filename_prefix("mclr")` and `filename_suffix("log")`, producing clean `mclr.YYYY-MM-DD.log` files in `%APPDATA%/MCLR/logs/`.
- **Unified Matcher (`is_launcher_log_file`)**: The shared utility `mclr_core::logging::is_launcher_log_file` matches both new `mclr.*.log` files and legacy `mclr.YYYY-MM-DD` files across:
  - `prune_old_logs`: Daily retention pruning (deleting logs older than configured retention period).
  - `compute_cache_sizes` & `clean_cache`: Toolbox log cache size calculation and clearing.
  - `launcher_logs_list`: Launcher log file browser on the `/logs?tab=launcher` panel.
- **Async IPC Access**: `launcher_logs_list` and `launcher_log_read` execute via `run_blocking` to avoid disk I/O on Tokio's async worker threads.

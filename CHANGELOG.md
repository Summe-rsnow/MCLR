# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.4] - 2026-10-24

### Added
- `mclr-auth`: DPAPI + AES-256-GCM encrypted account store (`accounts.enc`, `master.key`), Offline UUID generator (standard MD5 v3, legacy, custom), Microsoft 6-step device-code authentication flow with XBL/XSTS `XErr` mapping, Authlib-Injector + Yggdrasil Connect discovery/login, and 2D skin head cropper (`image` crate) + skin/cape APIs.
- `mclr-game::install`: Vanilla version manifest caching, client jar/libraries/natives/assets/log-config installer, and SHA1-verified incremental `repair_instance`.
- `mclr-cli install <mc> <name>` subcommand.
- Accounts UI (`src/features/accounts`) and Vanilla Install UI (`src/features/download`).

## [0.0.3] - 2026-10-24

### Added
- `mclr-net`: HTTP client with proxy and DoH (`hickory-resolver`), BMCLAPI + mcimirror URL rewrite, priority racing ladder (`mirrorFirst`, `smart`, `officialFirst`), and multi-connection chunked downloader (`Range` probe, 8 MiB chunks, dynamic slow-chunk splitting, SHA1/SHA256/MD5 verification).
- `mclr-game`: Minecraft version JSON parser, `inheritsFrom` merge, rule evaluation, instance scanner/classifier/CRUD (Recycle Bin deletion via `trash`), and Windows Java scanner (Registry, `PATH`, `JAVA_HOME`, standard directories, Mojang runtime manifest resolver).
- `mclr-cli java scan` subcommand.
- Task Manager overlay/page (`src/features/tasks`) with titlebar `TaskIndicator` and Instance List page (`src/features/instances`) with game-folder switcher and context menu actions.

## [0.0.2] - 2026-10-24

### Added
- `mclr-core` runtime modules: `paths` (`MCLR_DATA_DIR` override), `settings` (RFC 7386 merge patch, atomic write, 300 ms debounce), `secrets` (`.env` + process env lookup), `logging` (daily rolling log + retention pruning), and `TaskCenter` (hierarchical tasks, pause/resume/cancel cascade, `tasks://update` event).
- OKLCH tonal palette theme engine (`culori`) with `light`/`dark`/`system` modes and `flat` vs `glass` (liquid glass + SVG displacement refraction + Mica vibrancy) materials.
- Full 23-component `M*` UI library (`src/ui/`), custom frameless `AppShell`, and interactive `/dev/ui` showcase page.

## [0.0.1] - 2026-10-24

### Added
- Initial Cargo workspace (`src-tauri`, `mclr-core`, `mclr-net`, `mclr-game`, `mclr-auth`, `mclr-loaders`, `mclr-comp`, `mclr-saves`, `mclr-crash`, `mclr-update`, `mclr-cli`).
- Vue 3.5 + TypeScript + Vite 8 + Pinia + Vue Router + Vue I18n + UnoCSS frontend scaffold.
- `tauri-specta` IPC binding generation (`src/ipc/bindings.ts`) and `app_info` command.
- Repository documentation (`AGENTS.md`, `docs/*`), check gates (`scripts/check.mjs`, `scripts/lint-i18n.mjs`, `scripts/bump-version.mjs`).

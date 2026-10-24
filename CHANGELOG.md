# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

# MCLR — Minecraft Launcher Rewrite

**MCLR** is a modern, Windows-native Minecraft launcher built with **Tauri 2**, **Rust (Edition 2024)**, and **Vue 3.5 + TypeScript**. It re-implements the core feature set of Plain Craft Launcher 2 Community Edition (PCL-CE) on a clean, modular architecture with an original UI, OKLCH dynamic tonal palettes, and switchable Flat / Liquid Glass (Mica + refraction) materials.

## Features

- **Multi-Account Authentication (`mclr-auth`)**:
  - **Microsoft OAuth 2.0 Device Code Flow**: Full 6-step Xbox Live / XSTS / Minecraft Services chain with `XErr` diagnostics, skin upload, and cape switching.
  - **Third-Party Authlib-Injector & Yggdrasil Connect**: Automatic `X-Authlib-Injector-Api-Location` discovery, multi-profile selection, and OIDC support.
  - **Offline Accounts**: Standard RFC 4122 v3 MD5 (`OfflinePlayer:<name>`), legacy hash, and custom 32-hex UUIDs.
  - **Encrypted Storage at Rest**: Accounts stored in `%APPDATA%/MCLR/accounts.enc` (AES-256-GCM) with a 32-byte master key in `%APPDATA%/MCLR/master.key` protected by Windows DPAPI (`CryptProtectData`).
- **High-Speed Multi-Source Downloader (`mclr-net`)**:
  - Official Mojang / Modrinth / CurseForge CDNs + BMCLAPI + MCIMirror rewrites.
  - Configurable source racing ladder (`mirrorFirst`, `smart`, `officialFirst`).
  - HTTP `Range` chunked downloads (8 MiB chunks, dynamic slow-chunk work-stealing split, SHA1/SHA256/MD5 verification) and DNS-over-HTTPS (`hickory-resolver`).
- **Game & Mod Loader Management (`mclr-game`, `mclr-loaders`)**:
  - Vanilla installation and incremental SHA1-verified asset/library repair.
  - Automated installers for **Fabric**, **LegacyFabric**, **Forge** (legacy & modern headless `--installClient`), **NeoForge**, **Cleanroom**, **OptiFine** (standalone & Fabric/Forge combo), **LiteLoader**, and **LabyMod 4**.
  - Automatic Windows Java runtime scanner (Registry, `PATH`, `JAVA_HOME`, standard directories) and Mojang Java runtime downloader.
  - Full 9-stage launch pipeline with `@argfile` support for long classpaths on Java ≥ 9, native DLL management, real-time log sanitization, and custom window titles.
- **Community Resources & Modpacks (`mclr-comp`)**:
  - Unified search, project details, and version downloads across **Modrinth** and **CurseForge** for Mods, Modpacks, Resource Packs, Shaders, Data Packs, and Worlds.
  - DFS dependency resolver, local mod manager (`.jar` ↔ `.jar.disabled`, batch update checks), and nested **Jar-in-Jar** tree inspector.
  - Modpack import (CurseForge, Modrinth `.mrpack`, MultiMC/Prism, MCBBS, HMCL, nested packs, raw `.minecraft` zips) and `.mrpack` / CurseForge `.zip` export.
- **World Saves, Datapacks & Server List (`mclr-saves`)**:
  - `level.dat` NBT parser/writer across all DataVersion boundaries (`100`, `1443`, `2536`, `4774`), world backup/import/export, and datapack toggling.
  - `servers.dat` NBT management, `_minecraft._tcp` SRV resolution, RFC 8305 Happy Eyeballs, and Server List Ping (SLP) with one-click quick-play launch.
- **Crash Analysis (`mclr-crash`)**:
  - 4-phase heuristic analyzer covering all 45 crash causes with localized advice and one-click diagnostic `.zip` report export.
- **Personalisation, Home, Help & Tools**:
  - OKLCH seed-color palette generator, Light/Dark/System modes, Flat vs. Liquid Glass materials, image/slideshow/video (`<video muted loop>`) backgrounds, and background music player with auto-pause on game launch.
  - Minecraft news feed, remote announcements, bundled Markdown Help Center, skin fetcher, cache cleaner, custom downloader, `.lnk` desktop shortcut creator, and Daily Luck.
  - Self-updater (`mclr-update`) wrapping `tauri-plugin-updater`.

## Prerequisites

- **OS**: Windows 10 / 11 x64 (WebView2 Runtime installed)
- **Rust**: `1.99+` (MSVC toolchain `x86_64-pc-windows-msvc`)
- **Node.js**: `22+` with **pnpm** `12+`

## Runtime Secrets Configuration

MCLR reads optional service secrets at runtime in the following order:
1. Process environment variables
2. `%APPDATA%/MCLR/.env`
3. `<repo>/app/.env` (development)

Copy `.env.example` to `.env` and populate as needed:

```ini
MCLR_MS_CLIENT_ID=your-azure-ad-client-id
MCLR_CURSEFORGE_API_KEY=your-curseforge-api-key
MCLR_UPDATE_ENDPOINT=https://updates.example.com/mclr/latest.json
MCLR_UPDATE_PUBKEY=dW50cnVzdGVkIGNvbW1lbnQ6...
```

If a secret is not configured, MCLR never crashes; `secrets_status()` reports `false` and the UI displays a localized configuration notice (`error.notConfigured`) for that specific feature.

## Development & Build Commands

```bash
# Install dependencies
pnpm install

# Start frontend Vite dev server (with mock IPC in browser)
pnpm dev

# Start full Tauri desktop app in development mode
pnpm tauri dev

# Regenerate TypeScript IPC bindings from Rust (tauri-specta)
pnpm gen:bindings

# Run the full verification gate (fmt, clippy, cargo test, bindings, vue-tsc, vitest, i18n lint, vite build)
pnpm check

# Build production Windows NSIS installer
pnpm tauri build
```

## Headless CLI (`mclr-cli`)

```bash
# Print application info
cargo run -p mclr-cli -- info

# Scan installed Java runtimes
cargo run -p mclr-cli -- java scan

# Install vanilla or modded Minecraft instance
cargo run -p mclr-cli -- install 1.20.1 my-vanilla
cargo run -p mclr-cli -- install 1.20.1 my-fabric --fabric

# Launch an instance headlessly with an offline account
cargo run -p mclr-cli -- launch my-fabric --offline Steve

# Check for launcher updates
cargo run -p mclr-cli -- update check --channel stable
```

## License

Licensed under the [Apache License, Version 2.0](LICENSE).

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.9] - 2026-10-24

### Added
- `mclr-saves`: NBT `level.dat` reader/writer (`fastnbt` + `flate2`) supporting all DataVersion boundaries (`100`, `1443`, `2536`, `4774`), world save import/export/backup/delete, datapack scanner + toggle, screenshot and instance file (`resourcepacks`, `shaderpacks`, `schematics`) managers.
- `mclr-saves::servers`: `servers.dat` NBT CRUD, `_minecraft._tcp.<host>` SRV lookup + RFC 8305 Happy Eyeballs (250 ms stagger), and Minecraft Server List Ping (SLP `0x00` status + `0x01` RTT ping).
- Instance Detail page (`/instances/:id`) with glob-collected sub-tab registry (`src/features/instance-detail/tabs.ts`) and 9 sub-tabs: Overview, Settings, Mods, Resource Packs, Shaders, Schematics, Saves, Screenshots, and Servers (with "Launch into Server" quick-play).

## [0.0.8] - 2026-10-24

### Added
- `mclr-comp::modpack`: Modpack format inspector and importer for CurseForge (`manifest.json`), Modrinth (`.mrpack` / `modrinth.index.json`), MultiMC/Prism (`instance.cfg` + `mmc-pack.json`), MCBBS (`mcbbs.packmeta`), HMCL (`modpack.json`), nested launcher packs, and raw `.minecraft` zips, with Quilt modpack rejection (`modpack.quiltUnsupported`).
- `mclr-comp::local`: Local mod scanner parsing `fabric.mod.json`, `quilt.mod.json`, `META-INF/neoforge.mods.toml`, `META-INF/mods.toml`, and `mcmod.info`, CurseForge MurmurHash2 fingerprint calculation, `.jar` ↔ `.jar.disabled` toggling, and batch update checking.
- `mclr-comp::jarinjar`: Nested Jar-in-Jar tree analyzer (`fabric.mod.json`, `quilt.mod.json`, `META-INF/jarjar/metadata.json`, and 72-col unfolded `MANIFEST.MF`).
- `mclr-comp::resolver`: DFS mod dependency resolver (depth ≤ 32, installed-satisfaction check, channel priority `release > beta > alpha`).
- Local Mod Management UI (`src/features/mods`), JarInJar drawer, dependency resolution dialog in `CompDetailPage.vue`, and `ModpackImportDialog.vue`.

## [0.0.7] - 2026-10-24

### Added
- `mclr-comp`: Unified community project/file model (`CompProject`, `CompFile`, `CompType`), `ModrinthClient` (search, project, versions, batch projects, `version_files`, `version_files/update` with mirror rewrite), and `CurseForgeClient` (search, project, files, batch mods/files, `fingerprints/432`, null `downloadUrl` edge CDN fallback, and `comp.cf.distributionDisabled` handling).
- Community Download UI (`src/features/comp`): Mod, Modpack, ResourcePack, Shader, DataPack, World, and Favorites browser pages + Project Detail page with `markdown-it` + `DOMPurify` description rendering and target-instance download dialog.

## [0.0.6] - 2026-10-24

### Added
- `mclr-loaders`: `LoaderInstaller` trait and implementations for Fabric, LegacyFabric, LiteLoader, LabyMod 4, OptiFine (standalone & Fabric/Forge combo), Forge (legacy `install_profile.json` and modern headless `--installClient`), NeoForge, and Cleanroom.
- `mclr-cli install` loader flags (`--fabric`, `--forge`, `--neoforge`).
- Auto-install loader pickers with MC version compatibility filtering (Quilt hidden) and dedicated per-loader pages (`/download/fabric`, `/download/forge`, `/download/neoforge`, `/download/optifine`, etc.).

## [0.0.5] - 2026-10-24

### Added
- `mclr-game::launch`: Full 9-stage launch pipeline (path & login-requirement precheck, Java constraint selection, account session resolution, incremental repair, `.dll` natives extraction, JVM/game argument substitution with `@argfile` over 8000 chars for Java ≥ 9, `authlib-injector.jar` `-javaagent` setup, pre-launch command runner, `javaw.exe` spawn + `LaunchWatcher` with token/username masking, log severity classification, and Win32 `SetWindowTextW` custom window title).
- `mclr-cli launch <instance> --offline <name>` subcommand.
- Launch page (`src/features/launch`), virtualized Game Log viewer (`src/features/logs`), and Game/Launch & Java settings pages (`src/features/settings-game`).

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

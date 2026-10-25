# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.17] - 2026-10-25

### Added
- **UVMC Unlisted Version Manifest Merging**: Integrated UVMC unlisted versions (`https://alist.8mi.tech/...`) in `mclr-game::install::fetch_version_manifest` with local caching (`cache/uvmc_manifest.json`), achieving exact PCL version parity (112 releases, 825 snapshots, 165 ancient, 12 april fools, 1,114 total).
- **PCL-Styled Community Search Layout**: Reorganized `CompBrowserPage` with filter controls on top (platform 'all'/Modrinth/CurseForge, categories, sorts, target instance/MC version, loader) and full-width search input underneath; triggers search on Enter without dedicated button.
- **Interactive Favorites Feedback**: Updated favorite button with instant heart fill (`text-rose-500 fill-current`) and toast confirmations.
- **Reusable `MHelpTooltip` & Teleported Tooltips**: Created `MHelpTooltip` question-mark info badge; rewrote `MTooltip` with `<Teleport to="body">`, fixed coordinates, `z-[9999]`, and smart vertical flipping when near top edges to prevent clipping in headers.
- **Unified `MPathPicker` Component**: Created strictly non-editable path/file selection component with browse dialog trigger across settings, tools, java, and export flows.

### Changed
- **Select-to-Add Flow for Java and Game Folders**: Eliminated manual input and "+ 添加" button; selecting a file or folder in system dialog immediately adds and validates it.
- **In-List Folder Remarking & Management**: Folders in settings can be remarked inline, set active, opened, or deleted directly in the list.
- **Zero Default Game Directory & Guided Setup**: `folders` defaults to empty; prominent guidance banners and buttons appear in Launch, Instances, and Install when no folder is configured.
- **ToolsPage Masonry Packing**: Arranged tools into two vertical columns to eliminate empty gaps below Card 1.
- **UI Scale Dragging Decoupling**: Updated `MSlider` to commit font scaling on drag release (`@change`), eliminating screen jitter during adjustment.
- **Dynamic Segmented Content Sizing**: Updated `MSegmented` and `QuickSettings` so mode and material control backgrounds fit their content width instead of stretching into oversized capsules.

### Removed
- **Removed Daily Luck**: Completely deleted daily luck feature, methods, and UI cards from Tools.
- **Removed Official Java Downloads**: Removed Mojang Java runtime download section and RPCs from Java management page.
- **QuickSettings Cleanup**: Removed bottom divider and settings jump button from quick appearance popover.

## [0.0.16] - 2026-10-25

### Added
- **Unified Home & Launch with Fast News Toggle**: Integrated news & announcements directly below the launch view with an instant toggle switch (`mclr.launch.showNews`, default OFF). Skips background network requests when disabled.
- **Standalone Java Management Page**: Extracted Java settings from Settings into a dedicated first-class page (`/java`), positioned directly below Account Management in the navigation sidebar. Supports one-click scanning, Mojang official runtime downloads (Java 8, 17, 21), custom path validation, and architecture/type badges.
- **Simplified Help Center in Settings About**: Moved Help Center into Settings About & Help (`/settings/about`) with concise, collapsible FAQ accordions (Account login, Mod loaders, Java version selection, Crash diagnosis) and direct GitHub action links.
- **I18N Coverage & Dynamic Task Names**: Localized all Dev UI preview components, completed dynamic task identifiers in `tasks.json`, and standardized `*.nav` keys across all language bundles.

### Changed
- **Natural Card Heights in Grid Layouts**: Added `items-start` to `ToolsPage` and `AccountsPage` grids and eliminated forced `justify-between` / `h-full` stretching so cards size naturally to their own content.
- **Typography Streamlining**: Removed custom font family settings and scanner; standardizes on bundled HarmonyOS Sans SC while retaining UI scale adjustments.

### Removed
- **Removed Background Music**: Completely excised background music player, `useMusicPlayer`, `MusicMiniPlayer`, and audio elements from the UI shell and theme store.

## [0.0.15] - 2026-10-25

### Added
- **PCL-Parity Version Groups & Mock Manifest**: Reordered version accordions to match PCL (`最新版本` → `正式版` → `预览版` → `远古版` → `愚人节版`), populated comprehensive version list (112 releases, 825 snapshots, 165 old, 12 april fools), added Minecraft block-styled tiles for latest release/snapshot, and enhanced April Fools detection with Chinese lore descriptions.
- **Minecraft Block/Item Instance Icons**: Supported customizable Minecraft block and item icons (`grass_block`, `crafting_table`, `chest`, `furnace`, `diamond`, `sword`, `pickaxe`, `book`, `tnt`, `redstone`, `apple`, `command_block`, `ender_pearl`) rendered across Instances, Launch hero card, and Instance Detail views.
- **Base MC Version Badges & Mod Download Auto-Select**: Prominently displayed Minecraft base version on every instance card; auto-selected current instance and synchronized target MC version and loader filters in community mod downloads.
- **Custom Number Stepper**: Added custom theme-styled increment/decrement micro-buttons on `MInput` when `type="number"`, replacing unstyled browser spin buttons.

### Fixed
- **Sidebar Active Highlight Alignment**: Fixed sliding pill vertical offset calculation relative to `navContainerRef` to prevent offset misalignment from nested relative group wrappers; fixed route active matching for `/comp` subroutes.
- **Input Focus Highlight Outline**: Removed sharp rectangular browser `:focus-visible` outlines on inner `<input>` elements so focus states cleanly adhere to the rounded outer container.
- **Select & Checkbox Empty/Disabled States**: Provided clean placeholder text (`无可用选项` / `未选择`) and disabled styling for empty selections; prevented opening empty listbox popups.
- **Dropdown Scrollbar Corner Clipping**: Wrapped `MSelect` listbox in an outer `overflow-hidden` rounded container, eliminating scrollbar tracks sticking out of rounded corners.
- **Background Setting Preview Overlap**: Removed overlapping sample card when no background media is loaded; simplified background settings to single-image mode with direct clear ("X") button.
- **Component Width Adaptation & Shadow Truncation**: Protected `MSettingRow` labels with `min-w-[140px]` flex protection; eliminated harsh shadow cuts by adding margin/padding clearance around `MScroll` and virtual lists.
- **Dev UI Navigation Cleanup**: Moved UI component library viewer from sidebar navigation to a direct jump link in Settings Appearance live preview.

## [0.0.14] - 2026-10-25

### Added
- **Instance-First Empty Flow & PCL Card Classification**: Dev-only mock IPC isolation (zero demo instances in production bundles), automatic `ensure_game_folder` (`versions/` + `launcher_profiles.json`), empty-first Launch CTA ("下载游戏" → `/download`) and Instances empty state, 7-group PCL card classifier (`收藏`, `可安装 Mod`, `常规`, `不常用`, `愚人节版本`, `错误的实例`, `隐藏的实例`) with mtime-keyed scan cache, case-only rename support, and Recycle Bin / Shift-permanent deletion.
- **Staged Multi-Loader Create & Modify Pipeline**: `mclr-loaders::pipeline::install_instance` staging in `.mclr-staging` with `.mclr-installing` marker and `cleanup_stale` recovery, PCL merge order (`Vanilla → OptiFine → Forge → NeoForge → Cleanroom → LiteLoader → Fabric → LegacyFabric → LabyMod`), OptiFine-as-mod vs merged handling (incl. `mods/<mc>/` with LiteLoader), `cache/verified.json` parallel SHA1 repair cache, PCL `compat.ts` / `FolderNameValidator` rules, two-step `InstallPage.vue` + `InstallConfigurator.vue`, per-instance `InstallTab.vue` modify/reset flow, and `mclr-cli install` multi-loader flags.
- **Unified Per-Instance Resources & Community Target Resolution**: `mclr-comp::resources` for Mods, Resource Packs (`pack.mcmeta` flattening + `cache/packicons` thumbnail extraction), Shaders, and Schematics (gzip NBT inspection across `.litematic/.nbt/.schematic/.schem`), two-phase local mod scanning with 50 ms batched `resources://patch` streaming, `ResourceList.vue` (weighted search, filter chips, multi-select action bar, cascade dependency prompts, TXT/CSV export, drag-drop import), `ModsDisabledTab.vue`, and `comp_pick_target` download-into-instance priority.
- **Single Isolation Resolver & Progressive Media**: `mclr_game::gamedir::resolve_game_dir` unifying all 7 backend resolvers with 5-mode `DefaultIsolation` (`off | modded | nonRelease | moddedOrNonRelease | all`) and one-time `"global"` persistence, `instance_open_path`, Tauri `protocol-asset` scope + `assets_allow`, paged screenshots with 400 px disk-cached JPEG thumbnails (`screenshot_thumb`) and `arboard` clipboard copy (`screenshot_copy`), and world `iconPath` rendering.
- **Fluidity, Virtualization & Non-Blocking Runtime**: `ipc::run_blocking` across all sync IPC commands, shared `HttpState` `reqwest::Client`, `events::Batcher` (50 ms / 64 items for `tasks://update`, `launch://logBatch`, `resources://patch`), `createResource` stale-while-revalidate composable, `MVirtualList` / `MVirtualGrid` (`@tanstack/vue-virtual`), shimmer `MSkeleton` / `MSkeletonList`, `MCollapse`, `transitions.css` presets with `html[data-motion='full|reduced|off']` preference, sliding `SidebarNav` active pill, rAF-batched virtualized `LogsPage`, and lint rules (e)/(f) in `scripts/lint-ui-contracts.mjs`.

### Removed
- Removed global `/mods` route and `LoaderPage.vue` single-loader routes in favor of per-instance resource management and the unified two-step configurator.

## [0.0.13] - 2026-10-25

### Added
- **Tonal Layers Design System**: OKLCH-driven 5-tier surface hierarchy (`canvas`, `panel`, `raised`, `sunken`, `overlay`), WCAG-enforced foreground contrast (`ensureContrast` across 8 `SEED_PRESETS`), `AmbientBackdrop.vue` drifting radial blobs, and `GlassDefs.vue` SVG refraction filter (`#mclr-refract`).
- **Bundled Typography**: HarmonyOS Sans SC (`Regular 400`, `Medium 500 600`, `Bold 700` woff2) with license notices (`THIRD_PARTY_NOTICES.md`).
- **UI Component Library & Icons**: `MPageHeader`, `MSection`, `MSettingRow`, `MPopover`, `MBrandIcon` (`@iconify-json/simple-icons`), `MLoaderIcon`, custom teleported `MSelect` listbox, `ResizeObserver`-driven `MSegmented`/`MTabs` indicators, and `scripts/lint-ui-contracts.mjs` regression guard.
- **Grouped Navigation & Unified Settings**: Collapsible grouped sidebar (`SidebarNav.vue`, `TitleBar.vue`, `QuickSettings.vue`, `MusicMiniPlayer.vue`, `AccountChip.vue`) and merged `/settings` layout (`SettingsLayout.vue`) with vertical sub-navigation.

### Fixed
- Fixed missing navigation and component icons by scanning `.ts` files in `uno.config.ts` and declaring `icon`/`size`/`block` props across UI components.
- Fixed `presetWind4` color variable definitions so tonal surface and text utility classes render accurately in both light and dark modes.

## [0.0.12] - 2026-10-25

### Added
- `mclr-update`: Runtime self-updater wrapping `tauri-plugin-updater` v2 (`MCLR_UPDATE_ENDPOINT?channel=stable|beta` and `MCLR_UPDATE_PUBKEY` with `notConfigured` fallback), `mclr-cli update check` subcommand, and `--disable-gpu` WebView2 startup flag handling in `src-tauri/src/main.rs`.
- System Settings page (`src/features/settings-system`) with 7 tabs: Game Manage, Launcher Misc, Language (dynamic locale switcher), Update, Launcher Logs, Feedback, and About.
- Full i18n audit (`lint-i18n.mjs` + dynamic `zz-TEST` locale test), `README.md`, and NSIS installer packaging (`MCLR_0.0.12_x64-setup.exe`).

## [0.0.11] - 2026-10-24

### Added
- Appearance & Personalisation (`src/features/settings-appearance` & `AppShell.vue`): Seed colour picker + presets, theme mode, flat/glass material switch, image/slideshow/video (`<video muted loop>`) background with blur and opacity, Local Font Access API (`queryLocalFonts()`) + custom font input, UI scale slider, and `HTMLAudioElement` background music playlist (`mp3`/`wav`/`aac`/`flac`, loop/shuffle, volume, auto-pause on `launch://started`, auto-resume on `launch://exit`).
- Home & Help (`src-tauri/src/ipc/home.rs`, `src/features/home`, `src/help/zh-CN/`): Minecraft news fetcher with 30-minute disk cache and domain whitelist (`minecraft.net`, `minecraft-services.net`, `microsoft.com`), remote announcement banner with dismiss state persistence, and bundled Markdown Help Center.
- Tools (`src-tauri/src/ipc/tools.rs`, `src/features/tools`): Mojang player skin/avatar fetcher, cache/logs/skin cleaner, custom multi-connection downloader, Windows `.lnk` desktop shortcut creator (`IShellLinkW` launching `mclr.exe --launch <instance>`), and deterministic Daily Luck (0–100).

## [0.0.10] - 2026-10-24

### Added
- `mclr-comp::export`: Modpack exporter producing Modrinth `.mrpack` (`modrinth.index.json` formatVersion 1 with SHA1/SHA512 `version_files` resolution + `overrides/`) and CurseForge `.zip` (`manifest.json` with MurmurHash2 `fingerprints/432` resolution + `overrides/`), plus candidate file tree scanner with sane defaults.
- `mclr-crash`: 4-phase crash log analysis engine (`fatal`, `primary`, `stack/modindex`, `secondary`) porting all 45 PCL-CE crash causes, log collector/preparer, and diagnostic `.zip` report exporter (`launch-debug.bat`, `environment.json`, `mods.json` with JarInJar trees, `watcher-output.log`, and crash logs).
- Instance Detail Export sub-tab (`ExportTab.vue`) and global `CrashDialog.vue` auto-popup on `launch://crashed` with localized `zh-CN` titles and advice for all 45 crash causes.

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

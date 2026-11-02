# MCLR Historical Progress Archive (v0.0.1 – v0.0.31)

This archive consolidates the engineering slice plans and acceptance checklists from milestones v0.0.1 through v0.0.31.
For the high-level progress table and active release status, see [docs/progress.md](progress.md).

---



<!-- ===== v0.0.1.md ===== -->

# v0.0.1 — Foundation

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 1.1 | Git init, license, env example, docs | main | done | Files present |
| 1.2 | Cargo workspace + 9 crates + mclr-cli + src-tauri | main | done | `cargo test --workspace` |
| 1.3 | Vue 3.5 + Vite 8 + UnoCSS + vue-i18n + scripts | main | done | `pnpm check` |
| 1.4 | tauri-specta `app_info` + smoke proof | main | done | `cargo run -p mclr-cli -- info` & `pnpm tauri dev` |

## Notes / Blockers
- Embedded Windows Common-Controls v6 manifest into test binaries via `cargo:rustc-link-arg-tests` in `src-tauri/build.rs` so `tauri-plugin-dialog` (`rfd`) links `TaskDialogIndirect` cleanly during `cargo test`.

---


<!-- ===== v0.0.2.md ===== -->

# v0.0.2 — Core runtime + UI shell

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 2.1 | Core (`mclr-core`, `src-tauri/src/ipc/{app,settings,tasks}.rs`) | SliceCore | done | `cargo test -p mclr-core` |
| 2.2 | UI (`src/ui`, `src/stores/theme.ts`, `src/features/shell`, `src/features/dev-ui`) | SliceUI | done | `pnpm exec vitest run` |

## Shared IPC Signatures for v0.0.2

- `app_info() -> Result<AppInfo, AppError>`
- `secrets_status() -> Result<SecretsStatus, AppError>`
- `window_set_material(glass: bool) -> Result<(), AppError>`
- `settings_get() -> Result<Settings, AppError>`
- `settings_patch(patch: serde_json::Value) -> Result<Settings, AppError>`
- `tasks_list() -> Result<Vec<TaskNode>, AppError>`
- `task_cancel(id: String) -> Result<(), AppError>`
- `task_pause(id: String) -> Result<(), AppError>`
- `task_resume(id: String) -> Result<(), AppError>`

## Notes / Blockers
- All 7 `mclr-core` unit tests and 29 Vitest component/theme tests passed.
- Verified light/dark × flat/glass materials and OKLCH seed color palette updates in Chromium on `http://localhost:1420/#/dev/ui`.

---


<!-- ===== v0.0.3.md ===== -->

# v0.0.3 — Network, task UI, game model, Java

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 3.1 | Net (`mclr-net`: HttpClient, DoH, mirror rewrite, source racing, chunked Downloader + wiremock & ignored live tests) | SliceNet | done | `cargo test -p mclr-net` |
| 3.2 | Game (`mclr-game`: version JSON + inheritsFrom + rules, instance scan/classify/CRUD, Java scan/select/Mojang runtime download + `mclr-cli java scan`) | SliceGame | done | `cargo test -p mclr-game` & `cargo check -p mclr-cli` |
| 3.3 | UI + IPC (`src/features/{tasks,instances}`, `src-tauri/src/ipc/{instances,java}.rs`, commands, mocks) | SliceUI3 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- All 8 wiremock unit/integration tests in `mclr-net` passed, plus both `#[ignore]` live tests (`test_live_version_manifest` and `test_live_chunked_download_large_file` downloading 1.20.1 `client.jar` with chunked Range requests).
- `mclr-cli java scan` discovered GraalVM 25.0.3, GraalVM 21.0.11, and Adoptium JRE 1.8.0_482.

---


<!-- ===== v0.0.4.md ===== -->

# v0.0.4 — Accounts + vanilla install

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 4.1 | Auth (`mclr-auth`, `src-tauri/src/ipc/accounts.rs`: DPAPI+AES-256-GCM store, Offline, Microsoft 6-step + XErr + wiremock, Authlib + Yggdrasil Connect, skin/cape head crop) | SliceAuth | done | `cargo test -p mclr-auth` |
| 4.2 | Install (`mclr-game::install`, `src-tauri/src/ipc/install.rs`, `mclr-cli install <mc> <name>`) | SliceInstall | done | `cargo test -p mclr-game` & `cargo check -p mclr-cli` |
| 4.3 | UI (`src/features/accounts`, `src/features/download/install`, `src/ipc/client.ts`, mocks, tests) | SliceUI4 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified `mclr-cli install 1.20.1 t1`: installed `versions/t1/{t1.json,t1.jar}` + all libraries and assets; second run verified existing SHA1s and downloaded 0 files (`repairedCount: 0`).
- Offline account UUID for `Steve` verified as `"5627dd98e6be3c21b8a8e92344183641"` (RFC 4122 v3 MD5 of `OfflinePlayer:Steve`).
- Microsoft 6-step OAuth/XBL/XSTS/MC flow, `slow_down`, all 4 XSTS `XErr` codes, and `notConfigured` path verified via wiremock tests in `mclr-auth`.

---


<!-- ===== v0.0.5.md ===== -->

# v0.0.5 — Launch pipeline (first playable)

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 5.1 | Launch (`mclr-game::launch`, `src-tauri/src/ipc/launch.rs`, `mclr-cli launch <instance> --offline <name>`) | SliceLaunch | done | `cargo test -p mclr-game` & `cargo check -p mclr-cli` |
| 5.2 | UI (`src/features/launch`, `src/features/logs`, `src/features/settings-game`, `src/ipc/client.ts`, mocks, tests) | SliceUI5 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified `cargo run -p mclr-cli -- launch t1 --offline Steve`: spawned `javaw.exe`, extracted natives, masked Windows username with `***`, reached `[Render thread/INFO]: Setting user: Steve`, and exited cleanly with code 0.

---


<!-- ===== v0.0.6.md ===== -->

# v0.0.6 — Mod loaders

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 6.1 | `Fabric+Legacy+Lite+Laby+OptiFine` (`crates/mclr-loaders/src/fabric.rs`) | SliceFabric | done | `cargo test -p mclr-loaders --lib fabric` |
| 6.2 | `ForgeFamily` (`crates/mclr-loaders/src/forge.rs`, `src-tauri/src/ipc/loaders.rs`, `mclr-cli install --fabric/--forge/--neoforge`) | SliceForge | done | `cargo test -p mclr-loaders` & `cargo check -p mclr-cli` |
| 6.3 | `UI` (`src/features/download/` loader pickers + individual loader list routes, `src/ipc/client.ts`, mocks, tests) | SliceUI6 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified `mclr-cli install 1.20.1 f1 --fabric` and `mclr-cli launch f1 --offline Steve`: reached `[main/INFO]: Loading Minecraft 1.20.1 with Fabric Loader 0.20.1`.
- Verified `run_headless_installer_jar` (`--installClient` exit 0) and legacy Forge `install_profile.json` extraction + standalone JSON merging in `mclr-loaders` tests.

---


<!-- ===== v0.0.7.md ===== -->

# v0.0.7 — Community platforms

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 7.1 | Modrinth (`crates/mclr-comp/src/modrinth.rs`: search, project, versions, batch projects, `version_files`, `version_files/update` + wiremock & ignored live test) | SliceModrinth | done | `cargo test -p mclr-comp --lib modrinth` |
| 7.2 | CurseForge (`crates/mclr-comp/src/curseforge.rs`, `src-tauri/src/ipc/comp.rs`: search, mod, files, batch mods/files, fingerprints, null `downloadUrl` fallback + distributionDisabled) | SliceCurseForge | done | `cargo test -p mclr-comp` |
| 7.3 | UI (`src/features/comp` pages for Mod/Modpack/DataPack/ResourcePack/Shader/World/Favorites + Project Detail page with markdown-it + DOMPurify, `src/ipc/client.ts`, mocks, tests) | SliceUI7 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified wiremock tests for both `ModrinthClient` and `CurseForgeClient` (7 unit tests in `mclr-comp`).
- Verified live `#[ignore]` tests (`cargo test -p mclr-comp -- --ignored`): Modrinth search for `sodium` on `1.20.1`/`fabric` returned `sodium`; CurseForge live test verified `notConfigured` when `MCLR_CURSEFORGE_API_KEY` is absent.

---


<!-- ===== v0.0.8.md ===== -->

# v0.0.8 — Modpacks + local mod management

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 8.1 | Modpack (`crates/mclr-comp/src/modpack.rs`, `src-tauri/src/ipc/modpack.rs`: CF `manifest.json`, Modrinth `modrinth.index.json`, MultiMC/Prism `instance.cfg`+`mmc-pack.json`, MCBBS `mcbbs.packmeta`, HMCL `modpack.json`, nested launcher pack, raw `.minecraft` zip, Quilt rejection + fixture tests) | SliceModpack | done | `cargo test -p mclr-comp --lib modpack` |
| 8.2 | LocalMods (`crates/mclr-comp/src/{local.rs,jarinjar.rs,resolver.rs}`, `src-tauri/src/ipc/localmods.rs`: Fabric/Quilt/NeoForge/Forge/mcmod.info parser, MurmurHash2 + SHA1 identification, toggle `.disabled`, update check, JarInJar tree depth<=5/512 nodes/256 MiB, DFS dependency resolver depth<=32) | SliceLocalMods | done | `cargo test -p mclr-comp` |
| 8.3 | UI (`src/features/mods` mod list component + toggle/update/delete, JarInJar drawer, dependency dialog in `CompDetailPage.vue`, modpack import dialog in `CompBrowserPage.vue`/`InstancesPage.vue`, `src/ipc/client.ts`, mocks, tests) | SliceUI8 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified CurseForge pack, Modrinth `.mrpack`, and Prism/MultiMC zip fixture imports into expected isolated instance layouts (`isolation == "on"`), plus Quilt pack rejection (`modpack.quiltUnsupported`).
- Verified CurseForge MurmurHash2 whitespace-stripped fingerprint calculation, JarInJar nested tree parsing (including 72-column `MANIFEST.MF` line unfolding), and DFS dependency resolution (`depth <= 32`, `installed`-satisfied check, `Release > Beta > Alpha` channel ordering).

---


<!-- ===== v0.0.9.md ===== -->

# v0.0.9 — Instance management pages

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 9.1 | `Saves` (`crates/mclr-saves/src/saves.rs`, `src-tauri/src/ipc/saves.rs`: NBT `level.dat` across 4 DataVersion boundaries `100`, `1443`, `2536`, `4774`, datapacks, save zip import/export/backup/delete, resourcepacks/shaders/schematics/screenshots helpers) | SliceSaves | done | `cargo test -p mclr-saves --lib saves` |
| 9.2 | `ServerList` (`crates/mclr-saves/src/servers.rs`, `src-tauri/src/ipc/servers.rs`: `servers.dat` CRUD, SRV `_minecraft._tcp.<host>` + Happy Eyeballs 250ms stagger, SLP ping `0x00` handshake + `0x01` ping RTT against local fake TCP server in test) | SliceServers | done | `cargo test -p mclr-saves` |
| 9.3 | `InstancePages` (`src/features/instance-detail/{tabs.ts,InstanceDetailPage.vue,tabs/*}`, `src/features/instances/InstancesPage.vue` navigation, `src/ipc/client.ts`, mocks, tests) | SliceUI9 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified `level.dat` NBT round-trip across all 4 DataVersion boundaries (`< 100`, `100..1443`, `1443..2536` `DataPacks`, `2536..4774` `WorldGenSettings.seed`, `>= 4774` `spawn.pos` + `difficulty_settings`).
- Verified SLP ping (`0x00` handshake + status JSON + `0x01` ping/pong RTT) against a local fake TCP server and SRV `_minecraft._tcp.<host>` lookup with an injected resolver.

---


<!-- ===== v0.0.10.md ===== -->

# v0.0.10 — Modpack export + crash analysis

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 10.1 | `Export` (`crates/mclr-comp/src/export.rs`, `src-tauri/src/ipc/export.rs`: file tree scan with sane defaults, `.mrpack` `modrinth.index.json` + SHA1/SHA512 + `version_files` resolution + `overrides/`, CurseForge-style zip with `fingerprints/432`, round-trip test with v0.0.8 importer) | SliceExport | done | `cargo test -p mclr-comp --lib export` |
| 10.2 | `Crash` (`crates/mclr-crash/src/**`, `src-tauri/src/ipc/crash.rs`: log collector/preparer, 45 crash causes in `rules/*.rs`, diagnostic report zip export with `.bat` + env + mod list + JarInJar + logs, fixture tests for all cause families) | SliceCrash | done | `cargo test -p mclr-crash` |
| 10.3 | `UI` (`src/features/instance-detail/tabs/export.tab.ts` + `ExportTab.vue`, `src/features/crash/CrashDialog.vue` auto-popup on `launch://crashed`, `src/locales/zh-CN/{crash,instanceDetail}.json`, tests) | SliceUI10 | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified `.mrpack` export -> v0.0.8 modpack importer round-trip test (`test_modpack_export_roundtrip_mrpack`).
- Verified all 45 PCL-CE crash causes (`CrashCause::ALL.len() == 45`), 4-phase detection rules across 22 tests in `mclr-crash`, diagnostic report zip export, and `zh-CN` title/advice keys for all 45 causes.

---


<!-- ===== v0.0.11.md ===== -->

# v0.0.11 — Personalisation, home, tools

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 11.1 | `Theme/UI` (`src/features/settings-appearance/**`, `src/features/shell/AppShell.vue` background image/slideshow/video + music player auto-pause on `launch://started` and resume on `launch://exit`, font & scale) | SliceAppearance | done | `pnpm exec vitest run` |
| 11.2 | `Home+Help` (`src-tauri/src/ipc/home.rs`, `src/features/home/**`, `src/help/zh-CN/{index.json,*.md}`: Minecraft news fetch + 30 min cache + host whitelist, announcements + dismiss, bundled markdown help) | SliceHome | done | `cargo check -p mclr-app` & `pnpm exec vitest run` |
| 11.3 | `Tools` (`src-tauri/src/ipc/tools.rs`, `src/features/tools/**`: skin fetcher by name, cache cleaner, custom downloader task, `.lnk` desktop shortcut via `IShellLink`, daily luck 0–100) | SliceTools | done | `cargo check -p mclr-app` & `pnpm exec vitest run` |

## Notes / Blockers
- Verified Home (mock news + announcements), Help Center (bundled `zh-CN` markdown articles), Tools (skin fetcher, cache cleaner, custom downloader, `.lnk` desktop shortcut, daily luck), and Appearance settings (video background `<video muted loop>` + titlebar music controls) in Chromium.

---


<!-- ===== v0.0.12.md ===== -->

# v0.0.12 — Updates, remaining settings, i18n audit, packaging

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 12.1 | `Update` (`crates/mclr-update`, `src-tauri/src/ipc/update.rs`, `src-tauri/src/main.rs` `--disable-gpu`, `mclr-cli update check` + wiremock staged update test & `notConfigured` test) | SliceUpdate | done | `cargo test -p mclr-update` & `cargo check -p mclr-app` |
| 12.2 | `SettingsUI+i18n` (`src/features/settings-system/**`: Game Manage, Launcher Misc, Language, Update, About, Feedback, Launcher Logs + `zz-TEST` dynamic locale test + `README.md`) | SliceSettingsUI | done | `pnpm exec vitest run` & `node scripts/lint-i18n.mjs` |

## Notes / Blockers
- Verified `mclr-update` `notConfigured` path when `MCLR_UPDATE_ENDPOINT` / `MCLR_UPDATE_PUBKEY` are absent, and staged update manifest check (`?channel=stable` returning `0.0.99`) via wiremock test.
- Verified dynamic locale switcher test with throw-away `zz-TEST` locale in Vitest (100/100 tests passing).
- Verified `pnpm tauri build` produced Windows NSIS installer `target/release/bundle/nsis/MCLR_0.0.12_x64-setup.exe` (7.47 MiB).

---


<!-- ===== v0.0.13.md ===== -->

# v0.0.13 — UI Redesign: "Tonal Layers"

| ID | Slice | Owner | Status | Acceptance |
|---|---|---|---|---|
| 13.A | `Foundations` (`uno.config.ts` `.ts` pipeline + tonal theme, `src/ui/theme/{index.ts,fonts.css,material.css,glass.ts}`, `AmbientBackdrop.vue`, `GlassDefs.vue`, component library upgrade + `MPageHeader`/`MSection`/`MSettingRow`/`MPopover`/`MBrandIcon`/`MLoaderIcon`/`icons.ts`, `lint-ui-contracts.mjs`, `THIRD_PARTY_NOTICES.md`) | MainAgent | done | `pnpm vitest run src/ui` & `node scripts/lint-ui-contracts.mjs` |
| 13.B1 | `Shell` (`src/features/shell/{AppShell,TitleBar,SidebarNav,QuickSettings,MusicMiniPlayer,AccountChip}.vue`, `navGroups.ts`, `useBackgroundMedia.ts`, `useMusicPlayer.ts`, grouped route `NavMeta`) | Shell | done | `pnpm vitest run src/features/settings-appearance/__tests__/appearance.test.ts` |
| 13.B2 | `Settings` (`src/features/settings/{routes.ts,SettingsLayout.vue}`, unified `/settings/{appearance,game,system}` with `?tab=` sub-navigation, `MSection`/`MSettingRow` layout) | Settings | done | `pnpm vitest run src/features/settings-*` |
| 13.B3 | `Play` (`src/features/{launch,instances,instance-detail,logs,tasks,crash}` reskinned with `.m-page`, `MPageHeader`, `MSection`, `MLoaderIcon`, tonal status tokens) | Play | done | `pnpm vitest run src/features/{launch,instances,instance-detail,logs,tasks,crash}` |
| 13.B4 | `Discover` (`src/features/{accounts,download,comp,mods,home,tools}` reskinned with `.m-page`, `MPageHeader`, `MLoaderIcon`, `MBrandIcon`, `compTypeMeta`) | Discover | done | `pnpm vitest run src/features/{accounts,download,comp,mods,home,tools}` |

## Notes / Blockers
- Fixed UnoCSS missing `.ts` pipeline scan (`content.pipeline.include`) and `presetWind4` `<alpha-value>` color syntax so all route icons and tonal color tokens generate valid CSS.
- Bundled HarmonyOS Sans SC (`Regular`, `Medium`, `Bold` woff2) with `THIRD_PARTY_NOTICES.md` and About page attribution.
- Verified 57 browser screenshots across 14 routes × `{light, dark}` × `{flat, glass}` + `960x640` auto-collapsed sidebar rail (`68px`), WCAG contrast rules across all 8 seed presets (`palette.test.ts`), and `lint-ui-contracts.mjs` (0 violations).

---


<!-- ===== v0.0.14.md ===== -->

# MCLR v0.0.14 — Instance-first flow & fluidity

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1–A5 | Foundations (dev-only mock, asset protocol, `run_blocking`/`HttpState`/`Batcher`, `gamedir`, virtual/skeleton/motion primitives + lint e/f) | Main | done | `cargo clippy`, `vue-tsc`, `lint-ui-contracts` |
| B1 | Create & Modify instance flow (staged multi-loader pipeline, PCL compat/naming rules, two-step configurator + `InstallTab`, CLI `install`) | CreateFlow | done | `cargo test -p mclr-loaders -p mclr-game` + `pnpm vitest run src/features/download` |
| B2 | Instance cards/classification/caching, empty-first Launch & Instances pages, `InstanceDetailPage` tab transitions & `ModsDisabledTab` | Instances | done | `cargo test -p mclr-game` + `pnpm vitest run src/features/instances src/features/launch src/features/instance-detail` |
| B3 | Unified per-instance Resources (`mods/resourcepacks/shaderpacks/schematics`), two-phase mod streaming, delete global `/mods`, Comp download-into-instance | Resources | done | `cargo test -p mclr-comp` + `pnpm vitest run src/features/resources src/features/comp` |
| B4 | Progressive screenshots + disk thumbnails + clipboard, world icon paths, non-blocking launch/java/saves/servers/crash/logs | MediaBackend | done | `cargo test -p mclr-saves -p mclr-crash` + `pnpm vitest run src/features/logs src/features/settings-game` |
| B5 | Motion, skeleton, and controls sweep on Accounts/Tasks/Tools/Home/Settings/SidebarNav/TaskIndicator | MotionPages | done | `pnpm vitest run src/features/accounts src/features/tasks src/features/tools src/features/settings-appearance src/features/settings-system` |
| C1–C4 | Integration, bindings, live CLI + Chromium QA, docs, bump 0.0.14, commit & tag | Main | done | Full `pnpm check` + live QA |

## Notes / Blockers
- None.

---


<!-- ===== v0.0.15.md ===== -->

# MCLR v0.0.15 — UI Alignment, PCL Version Parity, Minecraft Icons & Component Polish

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1 | PCL-parity version groups & mock manifest (112 releases, 825 snapshots, 165 old, 12 april fools) | Download | done | `cargo test -p mclr-game` |
| A2 | Minecraft block & item instance icons (`MLoaderIcon`, block picker) | UI | done | `pnpm vitest run src/ui` |
| A3 | Base MC version badges & community mod download filter auto-selection | Instances | done | `pnpm vitest run src/features/instances` |
| A4 | Custom number stepper micro-buttons on `MInput` (`type="number"`) | UI | done | `pnpm vitest run src/ui` |
| A5 | Sidebar active highlight offset alignment & focus outline radius fixes | Shell | done | `pnpm vitest run src/features/shell` |
| A6 | Dropdown listbox scrollbar corner clipping & background settings preview overlap fixes | UI | done | Visual verification |

## Notes / Blockers
- None.

---


<!-- ===== v0.0.16.md ===== -->

# MCLR v0.0.16 — Home & Launch Consolidation, Standalone Java Page, Help in About & Cleanups

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1 | Consolidated Home & Launch page with fast news toggle (`mclr.launch.showNews`, default OFF) | Launch | done | `pnpm vitest run src/features/launch` |
| A2 | Standalone Java management page (`/java`) with registry/PATH scanning & validation | Java | done | `cargo test -p mclr-game` + `pnpm vitest run src/features/java` |
| A3 | Streamlined Help Center in Settings About with collapsible FAQ accordions | Settings | done | `pnpm vitest run src/features/settings-about` |
| A4 | Full i18n coverage & dynamic task keys audit | i18n | done | `node scripts/lint-i18n.mjs` |
| A5 | Natural card heights in grid layouts (`items-start`, no forced stretching) | UI | done | `node scripts/lint-ui-contracts.mjs` |
| A6 | Removed obsolete background music player, custom font settings & official Java downloader | Cleanup | done | `pnpm check` |

## Notes / Blockers
- None.

---


<!-- ===== v0.0.17.md ===== -->

# MCLR v0.0.17 — UVMC Parity, Select-to-Add Flows, MHelpTooltip, Compact UI & Polish

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1 | UVMC unlisted version manifest merging (1,114 total Minecraft versions parity) | Install | done | `cargo test -p mclr-game` |
| A2 | PCL-styled community search layout with top filter toolbar & Enter key trigger | Comp | done | `pnpm vitest run src/features/comp` |
| A3 | Interactive favorites heart feedback with toast notification | Comp | done | `pnpm vitest run src/features/comp` |
| A4 | Reusable `MHelpTooltip` badge & teleported `MTooltip` with edge boundary flipping | UI | done | `pnpm vitest run src/ui` |
| A5 | Unified non-editable `MPathPicker` component with native browse dialog | UI | done | `pnpm vitest run src/ui` |
| A6 | Select-to-add flow for Java and game folders with zero default folder setup guide | Settings | done | `pnpm vitest run src/features/java` |
| A7 | Removed Daily Luck feature & cleaned QuickSettings appearance popover | Cleanup | done | `pnpm check` |

## Notes / Blockers
- None.

---


<!-- ===== v0.0.18.md ===== -->

# MCLR v0.0.18 — Shell Redesign, News Masonry, Floating Paginator, Folders Dialog, MMarkdown & Skin Parity

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1 | Shell layout modernization (sidebar to top with logo, removed TitleBar dividing line) | Shell | done | `pnpm vitest run src/features/shell` |
| A2 | Multi-column news masonry (`columns-1 md:columns-2 lg:columns-3`) eliminating voids | Launch | done | `pnpm vitest run src/features/launch` |
| A3 | Unified floating paginator (`MPagination`) with dynamic page numbers & jumping | UI | done | `pnpm vitest run src/ui` |
| A4 | Dedicated game folders management dialog (`GameFoldersDialog`) | Instances | done | `pnpm vitest run src/features/instances` |
| A5 | High-fidelity skeleton matching across all 12 core pages & panels | UI | done | `node scripts/lint-ui-contracts.mjs` |
| A6 | Rich Markdown & HTML rendering component (`MMarkdown`) with DOMPurify sanitization | UI | done | `pnpm vitest run src/ui` |
| A7 | Keyless CurseForge access via MCIM mirror fallback | Comp | done | `cargo test -p mclr-comp` |
| A8 | PCL offline skin model resolution (Steve/Alex) & Authlib texture profile parity | Accounts | done | `cargo test -p mclr-auth` |
| A9 | Startup splash animation overlay with instant bypass toggle in settings | Shell | done | `pnpm vitest run src/features/shell` |
| A10 | Consolidated unified logs page (`/logs?tab=game|launcher`) | Logs | done | `pnpm vitest run src/features/logs` |

## Notes / Blockers
- None.

---


<!-- ===== v0.0.19.md ===== -->

# MCLR v0.0.19 — Tasks in Logs, Java Folder Reveal, MSelect Adaptive Width & Layout Refinements

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1 | Unified task management inside Logs page (`/logs?tab=tasks`) | Logs | done | `pnpm vitest run src/features/logs` |
| A2 | Standalone Java folder reveal via native `path_open` & custom removal | Java | done | `cargo test -p mclr-game` |
| A3 | News page jump fallback empty state with reset action | Launch | done | `pnpm vitest run src/features/launch` |
| A4 | Adaptive `MSelect` width based on selected option label | UI | done | `pnpm vitest run src/ui` |
| A5 | Community resource badges & version accordion spacing polish | Comp | done | `pnpm vitest run src/features/comp` |

## Notes / Blockers
- None.

---


<!-- ===== v0.0.20.md ===== -->

# MCLR v0.0.20 — Oxc Toolchain, NeoForge Detection, Simplified Instance Card & Liquid Glass Titlebar

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1 | Fast toolchain integration (Oxc `oxlint` & `oxfmt` with `pnpm lint` and `pnpm fmt`) | Toolchain | done | `pnpm lint && pnpm fmt` |
| A2 | Offline account default skin customization (Steve 4px classic / Alex 3px slim) | Accounts | done | `cargo test -p mclr-auth` |
| A3 | Liquid Glass frosted acrylic TitleBar on scroll with backdrop blur | Shell | done | `pnpm vitest run src/features/shell` |
| A4 | Enhanced NeoForge loader detection (FancyModLoader 1.20.1 & loader deduplication) | Loaders | done | `cargo test -p mclr-loaders` |
| A5 | Rich account select dropdown with avatars, mode badges, and right checkmarks | Launch | done | `pnpm vitest run src/features/launch` |
| A6 | Simplified instance cards (`h-36`, removed redundant badges and bottom divider) | Instances | done | `pnpm vitest run src/features/instances` |
| A7 | Eliminated horizontal layout shift without right blank gutters (`100vw` & `--sidebar-w`) | Shell | done | Visual verification |
| A8 | `MSelect` scrollbar vertical track margins (prevents corner clipping) | UI | done | Visual verification |

## Notes / Blockers
- None.

---


<!-- ===== v0.0.21.md ===== -->

# MCLR v0.0.21 — Unified Overlay Architecture, Codebase Health Overhaul & UX Consistency

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| 1 | Unified Overlay Architecture (`MOverlay`, `MDialog`, `MDrawer`, `confirmDanger`, `DevUiPage` demo) | UI / Shell | done | `pnpm vitest run src/ui src/features/dev-ui` |
| 2 | Codebase Health Overhaul (dead code purge, oversized file split, `MSearchBar`/`MStatusBadge`, Rust offload) | Backend / Frontend | done | `cargo test --workspace` + `pnpm check` |
| 3 | Enrich `LaunchPage.vue` instance card (`InstancePreviewCard` mirroring `AccountsPage` card) | Launch | done | `pnpm vitest run src/features/launch` |
| 4 | Unify `InstanceDetailPage` sub-tab widths to `1120px` (`.m-page`) & remove `MTabs` horizontal scrollbars | Instances | done | Visual verification + zero layout shift |
| 5 | Fix Launcher Logs list bug (daily `mclr.YYYY-MM-DD.log` suffix + `is_launcher_log_file` unified matcher) | Logging | done | `cargo test -p mclr-core` |
| 6 | Zero compiler, Clippy, Oxlint warnings & remove all warning suppressions (`#[allow]`, `eslint-disable`) | Quality | done | `cargo clippy -- -D warnings` + `oxlint` |
| 7 | Fix `pnpm tauri dev` `EBUSY` watcher errors via Vite `watch.ignored` on `**/target/**` & `**/crates/**` | DevShell | done | `pnpm tauri dev` clean startup |
| 8 | Reorganize and synchronize all Markdown documentation across root, `app/`, `app/docs/` & `app/progress/` | Docs | done | All Markdown files verified and synchronized |

## Release Gate Status
- **State**: Completed / Released.
- **Approval**: Explicitly approved and confirmed by user for v0.0.21 release.

## Notes / Blockers
- None. Wave execution is proceeding sequentially.

---


<!-- ===== v0.0.23.md ===== -->

# MCLR v0.0.23 — Design System Overhaul, Solid Flat Material, Rich Iconography & Universal MImage

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| 1 | 扁平模式扎实化与透明度根治 (AmbientBackdrop flat 下禁用彩色光斑，main 实色扎实层，TitleBar 规范) | UI / Shell | done | Flat 模式下无杂乱透明渗透，视觉纯粹扎实 |
| 2 | 浅色与深色色彩体系重构 (Canvas 层次、Panel 结构色、Raised 卡片边界、Sunken 输入色、双层真实投影) | Theme / Styling | done | 浅色模式通透高级、层次分明、对比度达到 WCAG AA/AAA |
| 3 | 真实 Minecraft 方块与官方加载器图标库及非真实占位符精简 (PCL 纹理 + 仅保留真实原版方块) | UI / Assets | done | 实例图标选择器纯正 MC 方块，告别假占位图标 |
| 4 | 下载中心紧凑化与组件大内边距根除 (对齐设置帮助紧凑无内边距卡片，行高 46px，移除臃肿 padding) | Download / UI | done | 下载中心紧凑高密度，全面消除各页面过大内边距 |
| 5 | 实例双分割线根除与 11 标签页 PCL 分组重构 (游戏 / 资源体系分组切换，模组管理对齐 PCL) | InstanceDetail | done | 彻底消除双线，标签按游戏/资源清晰组织，模组管理对齐 PCL |
| 6 | 全新统一 MImage 图片组件与全局替换 (带 error 自动重试、fallbackSrc/fallbackIcon 兜底、pixelated) | UI Primitives | done | 全局 17 处原生 `<img>` 全部替换为 `MImage`，零断图 |
| 7 | 全页面编排与高度组件化 (启动、实例、下载、账号、Java、工具、日志、设置标准化) | UI / Shell | done | 消除散落原始 div，全站页面结构一致性达 100% |
| 8 | 全量自动化门禁与规范校验 (Vitest 218 项全绿，Check Script，Lint UI Contracts 0 违规，Docs Sync) | Quality / Integrator | done | `pnpm check` 全绿，版本发布就绪 |

## Release Gate Status
- **State**: Released as `v0.0.23`.
- **Approved**: Explicitly authorized by user command ("当前升级只够作为0.0.23 请查看以往提交 提交0.0.23").

## Notes / Blockers
- Ready for commit and tag `v0.0.23`.

---


<!-- ===== v0.0.24.md ===== -->

# MCLR v0.0.24 — Sidebar Pill Scale Fix, Horizontal Overflow Elimination, Drawer Frosting & Mod Updates

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| 1 | 侧边栏高亮滑块缩放漂移根治 (监听 scale + ResizeObserver，消除缩放乱飘) | Shell | done | 0.8~1.25 缩放下滑块精准吸附活动项 (实测 delta=0) |
| 2 | 实例详情页横向滚动条消除 (main 增加 overflow-x-hidden，消除全局冗余横轴) | Shell / Detail | done | 实例详情页全界面 0 横向滚动条 |
| 3 | 修改实例界面底部被截断修复 (移除 Configurator 负外边距 -mx-4 -mb-4) | Download / Detail | done | 实例配置摘要与重置按钮完整展示，不被截断 |
| 4 | 侧边抽屉玻璃材质重构 (0.88 不透明度 + 28px 磨砂，去除 m-refract 扭曲) | UI / Styling | done | 抽屉玻璃扎实磨砂，杜绝廉价全透明感 |
| 5 | 模组资源卡片间距美化 (MVirtualList :gap="10"，卡片呼吸感充足) | Resources | done | 模组卡片具有清晰的 10px 上下间距 |
| 6 | 模组更新检测支持 (localModsCheckUpdates IPC 接入 + 工具栏“检查更新”按钮) | Resources / Backend | done | 点击“检查更新”成功拉取并展示最新可用更新 |
| 7 | 全量质量门禁与就绪验证 (Vitest 220 项全绿，Check Script，Lint UI Contracts 0 违规，Docs Sync) | Quality / Integrator | done | `pnpm check` 全绿，工作树就绪等待用户确认 |

## Release Gate Status
- **State**: Released as `v0.0.24`.
- **Approval**: Explicitly approved by user command ("正式提交0.0.24").

## Notes / Blockers
- None.

---


<!-- ===== v0.0.25.md ===== -->

# MCLR v0.0.25 — Java Defaults, PCL-Parity Mod Selection, Skin Avatar Generator & Core Polish

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| 1 | Java 默认版本支持与单项自动默认 (`select_java` 仅一项时直接默认 + 前端默认徽标/设为默认/取消默认) | Java | done | 只有一个 Java 时自动设为默认，多项时支持设为默认与恢复自动匹配 |
| 2 | PCL 模组贴图机制深度调研 (云端 Logo → JAR 内 mcmod.info/fabric.mod.json 贴图 → MD5 缓存 → 占位符 四级级联) | Research | done | 形成完备的本地与云端图片级联方案并归档 |
| 3 | 模组全选与整卡选中简化 (工具栏全选/取消选择按钮 + 点击卡片任意处选中，移除小复选框，PCL 左侧强调条) | Resources | done | 单击卡片即选中，全选/取消选择切换正常，无复选框 |
| 4 | 工具箱新增皮肤头像生成器 (`generate_head_avatar` Rust 算法 + `tools_generate_skin_avatar` IPC + 64/128/256/512 尺寸与保存) | Tools | done | 择皮肤文件按 PCL 算法放大面部并叠加帽子图层，可保存任意尺寸 PNG |
| 5 | 实例快捷操作健壮性 (重命名/复制输入即时清除错误提示) 与社区资源/启动诊断链路复核 | Instances / Comp | done | 输入非法名称后可即时纠正，安装与诊断链路无阻 |
| 6 | 全量门禁校验与就绪保持 (cargo test/clippy/fmt、vitest、vue-tsc、oxlint/oxfmt、lint-ui-contracts、lint-i18n、vite build) | Quality / Integrator | done | `pnpm check` 全绿 |
| 7 | 高级启动选项面板 (渲染器 / JVM 头部与推荐参数 / 游戏参数尾部 / 启动前命令 / 高性能显卡 / java.exe / 锁定内存 / 崩溃分析) + 设置 schema v2 迁移 | Launch | done | 全部选项真实生效，老配置自动补推荐 JVM 参数 |
| 8 | 加载器官方图标统一 (`MLoaderIcon` 浅色底片 + 官方 logo + `MSelect.image` 方形 logo) | UI | done | 安装向导、实例卡片、社区资源筛选均显示官方图标 |
| 9 | 加载器版本识别按 PCL 规则重写 (参数优先 + Maven 坐标按加载器分规则 + 去 classifier/去重) | Instances | done | 1.12.2 / 1.20.1 / 1.20.2+ / 26.3 全版本矩阵 11 项测试 |
| 10 | 实例自动命名修复 (切回原版更新、LabyMod 命名、撞名即时本地校验与错误 i18n) | Install | done | 撞名显示中文提示而非裸 key，不自动改名 |
| 11 | 启动页实例选择持久化修复 (自愈 watcher 在列表加载前清空 localStorage) | Launch | done | 重启后恢复上次选中的实例 |
| 12 | 浮层碰撞公共函数 `src/ui/floating.ts` + 接入 `MSelect`/`MTooltip`/`MPopover`/`MContextMenu`/`MToast` | UI | done | 下拉/提示/右键菜单/浮窗不越视口 |
| 13 | 窄窗口布局适配 (启动页选择器改堆叠、设置输入框收敛、`MSegmented` 标签按可用宽度折叠为图标) | Layout | done | 1002/820/720 三档零横向溢出 |
| 14 | 安装向导加载器区布局 (等分卡片、版本下拉内联、不兼容项直接隐藏、变更预览改悬停图标) | Install | done | 5 张卡片单行铺满，兼容性差的卡片不占位 |
| 15 | 图标名与焦点环健全性 (修复 15 个失效 lucide 名、统一帮助提示组件、裁剪容器补内边距) | UI | done | 151 个图标名全部存在，描边四边不裁切 |

## Release Gate Status
- **State**: Released (v0.0.25).
- **Approval**: User confirmed wrap-up ("收尾").

## Notes / Blockers
- None.

---


<!-- ===== v0.0.26.md ===== -->

# MCLR v0.0.26 — Launch Agents, PCL Parity Switches & Iteration Polish

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| 1 | LWJGL Unsafe 加速补丁 (`lwjgl-unsafe-agent.jar` 内嵌 + 仅对 LWJGL 3.4.1 注入 + 可禁用) | Launch | done | MC 26.1+ 实例启动参数含 `-javaagent:…lwjgl-unsafe-agent.jar`，其他版本不注入 |
| 2 | LegacyFix 兼容补丁 (`legacyfix.jar` 内嵌 + releaseTime < 2013-06-25 才注入 + 可禁用) | Launch | done | MC ≤ 1.5.x 注入，1.6+ 不注入 |
| 3 | 代理 jar 落地机制 (`<data>/tools/agents/` 首次使用时写出，写完改名，失败只告警不阻断启动) | Launch | done | 目录内两个 jar 字节数与原文件一致；写失败时启动继续 |
| 4 | 高级启动选项新增两行开关 + 提示 (禁用 LegacyFix / 禁用 LWJGL Unsafe) | Settings | done | 开关默认关，逐行有 ⓘ 说明，参数与后端字段联通 |
| 5 | 继承链覆盖 (合并父版本 JSON 后仍能识别 releaseTime 与 LWJGL 版本) | Launch | done | NeoForge 26.3 / Forge 1.5.2 这类无自描述的子 JSON 也能正确注入 |
| 6 | 文档与合规 (design-language 新增 7b 节、THIRD_PARTY_NOTICES 记录 PCL-CE 两枚 Apache-2.0 agent) | Docs | done | 说明 `禁用 JLW` 为何在本项目不存在 |
| 7 | 全量门禁 (cargo test/clippy/fmt、vitest、vue-tsc、oxlint/oxfmt、UI 契约、i18n、vite build) | Quality | done | `node scripts/check.mjs` 全绿 |
| 8 | 全库体检 (6 个只读审计 Agent 并行：Rust 启动/安装、Rust core/auth/net/comp/crash、UI kit、features、IPC 契约、文档与 i18n) | Audit | done | 汇总 2 个 P0 + 14 个 P1，逐条本地复核 |
| 9 | 安全修复：modpack `files[].path` 越界写入、Forge `maven/` zip-slip、`remove_mod_files` 路径越界 (统一 `mclr_core::safe_path::safe_join`) | Security | done | 遍历/绝对/盘符路径一律拒绝，含单测 |
| 10 | 数据保护：Create 失败不再删除既有实例目录、`.mclr-installing` 清理加 6 小时时效、损坏 config 备份为 `config.json.bak`、natives 短写删残文件 | Data | done | 4 条回归测试 |
| 11 | 功能修复：实例右键菜单失效 (`open`/`modelValue` 双属性)、LabyMod 安装未传选择、modify 安装 `removeModFiles` 字段名错、`resources://patch` 事件名不一致、日志行重复、OptiFine 安装失败被吞掉 | Feature | done | 逐条实测或单测覆盖 |
| 12 | 健壮性：`cleanup_stale` 误删、自动内存下限突破安全上限、崩溃报告 snake_case token 未脱敏 | Robustness | done | 相关测试全部更新为正确行为 |
| 13 | 前端：资源"添加文件"改走原生选择器 + `resources_import`、社区资源版本/加载器 chip 改用真实可选值、release 构建隐藏 dev/ui 入口、文件夹选择失败改为 toast、关键按钮补 aria 标签与 tab 语义 | Frontend | done | vitest 全绿，UI 契约 0 违规 |
| 14 | i18n：补齐 48 个缺失的后端错误码文案（现 83/83 全覆盖）、补齐 9 个任务标题 key | i18n | done | 脚本比对 Rust `code()` 与 `error.json` 零缺口 |
| 15 | 文档纠偏：README 微软登录标注为"后端已实现、入口暂屏蔽"、contracts 错误结构与 schemaVersion=2、architecture 补 `tasks.ts`、design-language 页边距、i18n 说明、PROGRESS 补 v0.0.22 缺口 | Docs | done | 逐条对照代码核实 |

| 16 | 桌面行为收敛 (release 构建拦截原生右键菜单/刷新/开发者工具快捷键，`devtools: false`) | Desktop | done | 生产包实测右键与 F5/F12 均被拦截，输入框保留粘贴菜单 |
| 17 | 高级启动选项语义统一为「开 = 启用」，可为默认开启的项全部默认开启 (显卡 / java.exe / 锁定内存 / LegacyFix / LWJGL Unsafe / 崩溃分析)，schema v4 迁移会把旧的 `disable*` 键反转为正向键 | Settings | done | 设置页七项开关状态符合预期，旧配置意图不丢 |

| 18 | CSP 启用：生产策略锁定脚本/连接/对象，远程图片按实际来源枚举（Modrinth / ForgeCDN / Minecraft+Microsoft 新闻域名），本地资源走 `asset:` 运行时授权；配套 `nosniff`/COOP/权限策略头与 5 条防漂移测试 | Security | done | 6 个页面在真实浏览器强制该策略下 0 违规；`cargo test -p mclr-app --test csp_test` 5 项通过 |

| 19 | IPC 路径校验：新增 `src-tauri/src/security.rs` 双层白名单（写/删限定在启动器目录，读/打开仅拒绝系统目录），覆盖 resources/screenshots/modpack/schematic/tools/assets/path_open | Security | done | 越界与系统路径两种拒绝场景各有单测 |

| 20 | Java 无全局默认：删除 `java.select`（schema v5 + 原始 JSON 清洗），Java 管理页移除设为/取消默认与单 Java 自动钉死 | Java | done | 旧配置加载不丢失其他项，且不再出现全局固定 Java |
| 21 | 按版本智能选 Java：精确大版本优先 → 最近的更高版本；低于要求一律不用；实例级指定不满足时回退；缓存未命中自动重扫；启动日志记录选中 Java | Launch | done | 1.21.1 → Java 21、26.3 → Java 25（真实启动验证） |
| 22 | 26.x 版本号映射：`26.3` / `26.4-snapshot` 需要 Java 25（`new_scheme_series`），修掉落到 Java 8/17 分支的 `UnsupportedClassVersionError` | Launch | done | 26.3 用 Java 25 启动成功 |
| 23 | 移除 Java 自动下载：删除 `java_download` 命令、Mojang runtime 解析、前端按钮与状态，改为「前往 Java 管理」一键跳转 | Java | done | 全库 0 处 `java_download` 引用 |
| 24 | Java 管理页与实例/账号页头部规范对齐（primary + ⟳ 图标按钮，JDK 偏好移入列表标题行），副标题恢复常规描述 | UI | done | 真实渲染对照截图 |
| 25 | 侧边栏选中态按材质适配：`.m-nav-pill`/`.m-nav-accent`（扁平=实色块、玻璃=半透明冲洗+模糊+高光/发光） | UI | done | 四种材质/明暗组合计算样式与截图核对 |
| 26 | 发布门禁：版本号 0.0.26、CHANGELOG、PROGRESS、progress 归档、tag | Release | done | `node scripts/check.mjs` 全绿 + `git tag v0.0.26` |

## Release Gate Status
- **State**: Completed / Released as `v0.0.26`.
- **Approval**: User explicitly requested the wrap-up ("收尾26版本").
- **Verification**: `node scripts/check.mjs` (fmt + clippy -D warnings + cargo test workspace + vue-tsc + vitest + oxlint/oxfmt + UI contracts + i18n + vite build) green; live launches verified for 26.3 (Java 25) and 1.21.1 NeoForge (Java 21).

## Notes / Blockers
- `禁用 JLW` 有意不实现：MCLR 以 argv 数组直接启动 java，不使用 JavaLaunchWrapper，无对应机制可禁。

---


<!-- ===== v0.0.27.md ===== -->

# MCLR v0.0.27 — 流畅性 / CPU / 内存（渲染管线等价改写 + 无效工作消除）

目标：去掉每帧重栅格化、强制同步布局、paint 动画与 O(n) 日志冲刷，界面视觉零变化。
所有 CSS/动画改写都用「同会话 A/B 像素比对」证明等价（见下面的 P 段证据）。

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| A1 | 环境背景：滚动期间暂停 40s 漂移（250ms 无操作后恢复） | UI | done | 静态渲染与 v0.0.26 逐像素相同（A/B mean/max = 0） |
| A1b | 环境背景去掉容器 `filter: blur(60px)` | UI | **reverted** | 像素比对各相位 mean 0.674 > 门限 0.4 → 按方案回退，保留 `filter` |
| A2 | 骨架屏流光从 `background-position`（paint）改为 `translateX`（合成） | UI kit | done | 8 相位 × 明暗两套比对：mean 0.256 / p99 1.333 / max 6（门限 0.4 / 2 / 6） |
| A3 | `transition-all` 收敛为显式属性表（`MProgress` / `MCard` / 侧栏胶囊） | UI kit | done | 真实 hover 变更属性 ⊆ `transition-property`；唯一未覆盖项是 `--un-*` 自定义属性（本来也不参与过渡） |
| A4 | 主题写入合并到单帧 + 调色板记忆化（LRU 8） | Theme | done | `documentElement.style` 全键值 dump 与 v0.0.26 字符串完全相等（明/暗） |
| A5 | 任务进度 tween 改为「持续 rAF + 目标值」模型，不再每次事件重启 | Tasks | done | 单帧只调度一次；`\|target-current\| < 0.5` 吸附并停 rAF |
| A6 | 收藏判定改 Set 记忆化 + 版本分类按版本 id 缓存 | Comp/Download | done | 收藏键 = 每条收藏生成 `platform:projectId` 与 `platform:slug` 两项；过滤延迟不变（无防抖） |
| A7 | 侧栏胶囊几何更新 rAF 节流 | Shell | done | 滚动后 `pillTop`/`pillHeight` 与 v0.0.26 相同（rectTop 88.5 / rectH 36 / style top 32.5px） |
| B1 | 日志 `shallowRef` + 自增 `seq`，冲刷整体替换数组（保持原 rAF 节奏） | Logs | done | 无 `push(...)` / 原地 `splice`；`LOG_CAP` 10000 不变 |
| B2 | 过滤增量化（`filterKey` 不变时只处理新增行，裁剪二分） | Logs | done | 追加 20 行且过滤不变 → `filteredLogs` +20 且旧条目 `toBe` 同一引用 |
| B3 | 终端滚动写回单次 rAF（一次 `scrollHeight` 读 + 一次写） | Logs | done | watch 源改为 `flushTick`；无二次强制布局 |
| B4 | 日志测试同步点 + 3 条性能/引用稳定性用例 | Logs | done | 10000 行冲刷 < 50 ms |
| C1 | 启动走 `.mclr-verified.json` 快路径（`RepairMode::Fast`） | Install | done | 伪造指纹后删掉 asset index 仍返回 `Ok(0)`；删指纹后必须走全量（返回 `Err(Net)`） |
| C2 | `specs.rs` 同步 IO 全部搬进 `spawn_blocking` | Install | done | `collect_version_download_specs` / `verify_specs_cached` 保持 async 签名 |
| C3 | asset index 解析记忆化（按绝对路径 + mtime，>4 条清空） | Install | done | 命中复用 `Arc`，mtime 变化即失效 |
| C4 | 下载扇出改固定工作池 + `JoinSet`，删除 50ms `is_finished` 轮询 | Net | done | 200 个 spec、`max_concurrent_files=4`，实测并发峰值 4，全部落盘 |
| C5 | 小文件边下边算摘要，免二次整读；已知 size 不再探测 Range | Net | done | `test_hash_mismatch_retry` 仍绿（不匹配仍重试）；206 缺失时回退单流 |
| C6 | Modrinth 按 100 个一批查询 sha1 → project_id | Comp | done | 列表循环内 0 次单条请求 |
| C7 | 删除第二份缓存（`LOCAL_MOD_CACHE` / `localmods.json`），统一 `scan_local_mods` | Comp | done | `LocalModCacheItem` 增加 `cached_at_secs`；旧文件不再读取 |
| C8 | 扫模组不再整包读入：`parse_mod_metadata_path` + 流式 sha1/ murmur2 | Comp | done | 12 组 fixture（含 64 KiB±5）reader ≡ slice；sha1/murmur2 与旧实现逐字节相等 |
| C9 | `java_scan` 缓存非空时直接返回，`JAVA_SCAN_IN_FLIGHT` 防重复扫描 | Misc | done | 无 `java://updated` 消费者，删除后台发射无可见影响 |
| C10 | 实例排序不再在比较器里做 IO | Misc | done | 每实例一次 `resolve_game_dir`/`metadata`/`read_dir`，排序键不变 |
| C11 | 截图分页缓存目录扫描（目录 mtime 失效） | Saves | done | 重复调用分页结果一致；新增文件后自动重扫 |
| C12 | 存档图标不再内联 base64 | Saves | done | `SaveInfo.icon_data_url` 删除；bindings/mock/SavesTab 同步 |
| C13 | 启动器日志只读尾部 200 KB | Misc | done | 5 MB 文件 → 恰好 200 KB 且内容为末尾片段 |
| C14 | Settings 改 `RwLock<Arc<Settings>>` + `snapshot()` | Core | done | `get()` 保留并委托；`ipc/mod.rs`、`resources.rs`、`java.rs` 热路径全部改 `snapshot()` |
| C15 | 实例扫描锁只护缓存读写 | Instance | done | 遍历/解析/分类全在锁外，两次短临界区 |
| C16 | 加载器安装子进程走 `spawn_blocking` | Loaders | done | Forge/NeoForge/Cleanroom/OptiFine 四处 `output()`/`status()` |

| U1 | 检查更新改为显式占位：仅当存在签名更新源时才可点，否则禁用按钮 + `敬请期待` 徽标 | Update | done | `secrets_status` 缺 `updateEndpoint`/`updatePubkey` 时不发请求、不出现红色错误框 |
| U2 | 关于页仓库与反馈入口改为占位 | About | done | 原硬编码 `github.com/mclr-dev/mclr`（不存在的仓库）删掉；仓库/Issues 按钮禁用 + `敬请期待` |
| U3 | 启动自动检测与侧栏红点移除 | Shell | done | 无更新源时不应假装在检查；相关 store/测试一并删除 |
| U4 | 错误文案占位符修复 | Error | done | `formatError` 把 `detail` 注入插值参数，`{detail}` 不再原样显示 |
| U5 | 设置新增“赞助支持”页（置于子导航最底部） | Settings | done | `/settings/sponsor`；两个收款码走 `public/sponsor/*.png`（CSP `img-src 'self'`）；仅展示扫码，无保存/解锁类承诺 |
| U6 | 移除项目许可证声明 | Legal | done | 删除 `LICENSE`、`Cargo.toml` 的 `license` 字段（含 11 个 crate）、`package.json` 的 `license`、关于页徽标；README 改为“暂无许可证”；第三方组件的许可保留在 `THIRD_PARTY_NOTICES.md` |

## 视觉零变化证据（同会话 A/B，注入旧实现）

采集方法：`pnpm dev`（`VITE_MOCK_IPC=1`）+ Chromium，用 Web Animations API
（`animation.pause(); animation.currentTime = t`）把动画冻结到确定相位，再逐像素比对。
对照组（同一实现采两次）差异恒为 **0**，即该方法无噪声。

| 项 | 采样 | 结论 |
| --- | --- | --- |
| A1b 去容器 blur | 明/暗，4 个漂移相位 | mean 0.674、p99 3、max 6 → 超出 mean ≤ 0.4 门限，**回退** |
| A1 回退后的当前实现 vs 原实现 | 明，隔离背景层 | mean 0 / max 0 → 完全一致 |
| A2 骨架屏流光 | 明/暗 × 13 相位 | 最差相位 mean 0.256、p99 1.333、max 6 → 通过 |
| A3 过渡属性 | 真实 hover + 计算样式全量 dump | 变更属性只有 `translate`、`--un-translate-y`、`--un-shadow`，前者在 `transition-property` 内 |
| A4 主题终态 | 明/暗 `documentElement.style` | 与 v0.0.26 字符串完全相等 |
| A7 侧栏胶囊 | 滚动前后 | `rectTop 88.5` / `rectH 36` / `top 32.5px`，与 v0.0.26 相同 |

## 附带修复

- `launch/agents.rs::ensure_agent` 的临时文件名改为「进程 id + 自增计数」唯一，并在 rename 失败时
  回退检查目标文件是否已就位。此前并发首次落地会互相截断同一 `.tmp`，导致
  `agents_apply_to_instances_that_inherit_their_parent_json` 约 1/5 概率失败（修复后连跑 8 次全绿）。
- `mclr-saves` 不再需要 `base64` 依赖，随 C12 一并从 crate 依赖与 `Cargo.lock` 移除。

## 偏差 / 未做

- **A1b（去 blur）按方案回退**：`AmbientBackdrop` 仍保留 `filter: blur(60px)`，只保留「滚动期间暂停漂移」。
  代价是 glass 材质下背景层动画仍在每帧重栅格化；若日后允许 2.4% 以内的单像素亮度差异，可单独立项。
- **A5 曲线**：由 300 ms 缓动 tween 变为指数收敛（0.25/帧，约 12 帧到位），未在真实下载流上采样 P6。
- **A6 收藏键**：每条收藏生成「platform:projectId」与「platform:slug」两个键（单键无法回答只带一个标识的查询）。
- **C8 murmur2**：`compute_curseforge_murmur2_reader` 仍保留一份过滤后缓冲——种子为 `1 ^ filtered_len`，
  `h ← (h*m) ^ k` 不可分块折叠，`&mut impl Read` 无法零拷贝单趟；结果与旧实现逐字节相等。
- **C11 缓存取舍**：就地覆盖同一文件名的截图不会改变目录 mtime，需等到下一次增删才刷新 size/时间。
- **C2 `verify_specs_cached`**：`JoinError` 无 `Result` 可返回，改为 `error!` + 空集合（与既有容错方向一致）。

## Release Gate Status

- **State**: Released as `v0.0.27` (version bumped, CHANGELOG/PROGRESS archived, tag pushed).
- **Approval**: User explicitly requested the wrap-up ("收尾27版本").
- **Verification**: `node scripts/check.mjs` green — fmt, clippy `-D warnings`, workspace
  tests, bindings idempotency, oxlint/oxfmt, vue-tsc, vitest, i18n, UI contracts, vite build.
- **Browser smoke** (`VITE_MOCK_IPC=1`): `/logs` `/comp` `/download` `/settings/*` `/instances`
  `/` `/tools` all render with zero non-network console errors; log search filters live;
  sponsor page and the 敬请期待 placeholders confirmed in light and dark themes.
- **Not measured**: real Tauri/WebView2 idle CPU and scroll frame timing (the browser harness
  cannot represent the WebView2 compositor), and a live `Repair: skipped (verified manifest)`
  launch (no Minecraft instance exists on the build machine) — the fast path is covered by
  `crates/mclr-game/tests/repair_fast_path.rs`.

---


<!-- ===== v0.0.28.md ===== -->

# MCLR v0.0.28 — 整合包=独立实例 + 实例资源页模组化

目标：社区资源的「整合包」彻底脱离「先选一个实例」的心智模型（解包→补全→得到一个实例），
实例详情「资源管理」把模组放到第一个标签页，并让模组显示自己的封面。

| ID | Slice | Owner | Status | Acceptance |
| --- | --- | --- | --- | --- |
| M1 | 社区资源·整合包标签页不再继承目标实例的版本/加载器过滤，切页时清空 | Comp | done | `/comp/modpack` 不出现「目标实例」与「游戏版本」选择；由「模组」页带过来的 `1.20.1 / Fabric` 过滤被清空，搜索结果不受已装实例限制 |
| M2 | 快速下载弹窗：整合包忽略实例、按「全部版本」起步，加载器过滤对整合包不生效 | Comp | done | 有 Fabric 1.20.4 实例时打开 1.20.1 整合包，文件列表仍完整（改前会被实例过滤清空）；弹窗无目标实例下拉 |
| M3 | 整合包安装「解包后补全」的结果在任务真正结束后才报告，并刷新实例列表 | Comp/Tasks | done | `modpackInstallRemote` / `modpackImportFile` 返回的 taskId 经 `tasks://update` 等待终态；失败/取消报错，成功才 `fetchInstances(true)` |
| M4 | 实例详情·资源管理：模组为第一个标签页 | Instance detail | done | 标签注册顺序 `mods → saves → screenshots → resourcepacks → shaders → schematics → servers`；切到「资源管理」默认落在「模组」 |
| M5 | 模组封面：从 jar 内提取图标并缓存，随 `resources_list` 与流式 patch 下发给行组件 | Comp | done | Fabric/Quilt `assets/<id>/icon.png`、Forge/NeoForge `logoFile`、根 `logo.png`/`icon.png`/`pack.png` 依次回退；真实实例 93 个模组中 72 个出图；缓存为可解码 PNG（`crates/mclr-comp/tests/mod_icon_cache_test.rs`） |
| M5b | jar 内无图时用 Modrinth 项目图标兜底（`v2/projects?ids=[…]` 批量取 `icon_url`） | Comp | done | 不使用拼接的 `/data/<id>/icon.png`（CDN 实际是内容哈希的 `_96.webp`，猜路径会 400）；仅对缺图且已解析出项目 id 的模组批量请求一次，失败则该行退回占位图标 |
| M6 | 缓存目录语义统一：`cache/packicons` → `cache/icons`，包图标与模组图标共用 `cache_icon_png` | Comp | done | 资源包与模组图标都落在 `cache/icons/<sha1(key)>.png`；旧目录自动被忽略并重建 |
| M7 | 封面加载不阻塞列表：占位图标先绘制，图片解码完成后再淡入覆盖 | Resources | done | 图标位叠「静态占位 + `MImage`（`opacity-0 → 100` on load）」，无 src、加载中、加载失败三种状态都有可见内容 |
| M8 | 实例详情分组顺序：资源管理在前、游戏管理在后；页面默认落在「模组」 | Instance detail | done | 分段控件顺序 `资源管理 → 游戏管理`；打开实例直接显示模组列表（原版实例显示不可用提示） |
| M9 | 封面取图修正：方形优先 + 缓存版本 + 像素图最近邻 | Comp | done | YUNG's 系列 jar 同时有 `logo.png`(1200×240 横幅) 与 `icon.png`(256²)，改前取横幅被裁切；现按 IHDR 尺寸挑最大方形，10 个模组缓存全部 128×128；`localmods.json` 旧条目凭 `icon_rev` 自动重取；≤64px 原图走 `image-rendering: pixelated` |
| M10 | 行卡片对齐 PCL：分类标签移到第二行并按语言翻译，后接 `文件名 · 大小 · 描述` | Resources | done | 分类来自 Modrinth 项目元数据（与取图标同一次批量请求），`comp.search.categories.*` 有译文则翻译；标签去重且最多 4 个 |
| M11 | 图标在写入缓存时**预缩放到显示尺寸**，界面不再放大图片 | Comp | done | 统一 80×80（40 px 磁贴的 2×），**按短边缩放再居中裁掉长边溢出**，磁贴始终填满、无黑边/letterbox：≤64 px 像素图 Nearest、其余按缩放方向选 Triangle/CatmullRom；`ICON_SIZE`+`RENDER_REV` 参与缓存键，`ICON_CACHE_REV=4` 让旧缓存自动重建；行内 `fit="cover"`；`test_icon_is_cropped_to_fill_not_letterboxed` 覆盖 4:1 与 1:1 两种源 |

| M12 | 封面获取对齐 PCL：**在线项目 Logo 优先**，jar 内图兜底；并读取 jar 内声明的图标路径 | Comp | done | PCL `ModLocalComp.GetLogo()` = `Comp.LogoUrl` → jar `Logo` → NoIcon；本实现同序：Modrinth `icon_url` → jar 图 → 占位。同时新增 `fabric.mod.json.icon` / `quilt_loader.icon` 声明路径（原来只认 Forge 的 `logoFile`）。实机 `1.21.1-technology`：65/75 拿到 Modrinth Logo（256²/96²/64²），4 个仅 jar 有图，6 个无图 |

| M13 | 依赖全量升级（前端 + Rust，含跨大版本） | Infra | done | 前端：`pinia 4`、`vue-router 5`、`markdown-it 15`、`vitest 5`、`happy-dom 21`、`vue-tsc 3`、`unocss 66.12`、`oxlint 1.90`、`oxfmt 0.75`、`@types/node 26`。Rust：`reqwest 0.13`、`hickory-resolver 0.26`（改用 `Resolver` + `TokioRuntimeProvider`，DoH 名服务器构造变更）、`zip 9`（`entry.name()` 返回 `Result<Cow<str>>`，`Lookup` 改为 `.answers()` + `RData::SRV` 字段访问）、`rand 0.10`、`aes-gcm 0.11`（`Nonce::try_from`/按引用传 nonce）、`windows 0.62`（`BOOL` 移到 `windows::core`）、`tauri-specta rc.26` + `specta rc.26` + `specta-typescript 0.0.13`（导出器禁止 bigint → 用 `dangerously_cast_bigints_to_number`，`Result` 类型改由前端自定义）、`toml 1.1`、`dirs 7`、`winreg 0.56`、`window-vibrancy 0.8`、`base64 0.23`、`sha1/sha2/md-5 0.11`。TypeScript 锁 `^5.9.3`（vue-tsc 尚不兼容 TS 7 的包布局）。全门禁绿 + 真机冒烟（7 条路由 0 控制台错误，社区搜索经新 reqwest/hickory 仍返回 10806 条） |

## 设计说明

- **为什么整合包不需要目标实例**：整合包自带 `mcVersion` + `loader`，导入流程本身就会
  `install_vanilla` / `install_with_loader` 补全游戏与加载器，产物就是一个新实例
  （`versions/<name>` + `mclr.instance.json` 强制 `isolation = on`）。
  用已装实例的版本/加载器去过筛，只会把候选文件筛成空列表。
- **模组封面来源（与 PCL `ModLocalComp.GetLogo()` 同序）**：在线项目 Logo → jar 内图 →
  占位。在线来源是 API 返回的 `icon_url`（CDN 文件名带内容哈希，不能拼路径）。jar 图在
  `scan_local_mods` 缓存未命中时解码一次并归一化为 80×80（短边缩放 + 居中裁切），之后 UI
  只做缩小；`ICON_CACHE_REV` / `ICON_SIZE` / `RENDER_REV` 共同参与缓存键，规则一变旧文件自动失效。
- **图标容器的坑**：`MImage` 根节点自带 `relative`，用 `absolute inset-0` 给它定位会失效
  （UnoCSS 里 `.relative` 后置），图片按原始像素渲染再被 40px 磁贴裁成左上角 —— 这是
  “图片看起来一直不对”的真正原因；现在改成显式 `w-10 h-10`。
- **补全覆盖**：无 `fabric.mod.json`/`mods.toml`/`mcmod.info` 的 jar 仍会尝试根目录的
  `logo.png`/`icon.png`/`pack.png`，因此很多老模组也能出图。

## Release Gate Status

- **State**: Released as `v0.0.28` (version bumped, CHANGELOG/PROGRESS archived, tagged).
- **Approval**: 用户确认收尾（「收尾吧」）。
- **Verification**: `node scripts/check.mjs` 全绿 — fmt、clippy `-D warnings`、workspace
  tests、bindings 幂等、oxlint/oxfmt、vue-tsc、vitest 268、i18n、UI contracts、vite build。
- **Real-app smoke (WebView2 via CDP)**: `/launch`、`/instances`、`/instances/<id>`（模组图标
  与分类）、`/comp/modpack`、`/comp/mod`（10806 条，走新 reqwest/hickory）、`/download`、
  `/settings/*`、`/logs`、`/tools` 渲染正常，控制台 0 错误。
- **Not measured**: 真实模组包从社区下载到“补全完成”的端到端耗时；本轮只验证了任务终态
  回调与实例列表刷新，没有在真机上下载一个新整合包。

---


<!-- ===== v0.0.29.md ===== -->

# MCLR v0.0.29 — 性能档位滑块 + 系统托盘后台留存

目标：外观页多一个「性能 → 均衡 → 质量」的三档滑块，一键设置材质/动画/启动动画（原来三项
全部保留，滑块只是更省事的入口）；并让启动器可以关闭窗口后留在系统托盘继续跑。

| ID  | Slice                                                                           | Owner               | Status | Acceptance                                                                                                                                                                                                                                                                                                                 |
| --- | ------------------------------------------------------------------------------- | ------------------- | ------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1  | 新增「性能档位」三档滑块（快捷预设），原有材质/动画/启动动画三行全部保留        | Settings-appearance | done   | 外观页顺序为 强调色 → 色彩模式 → 性能档位 → 高级材质效果 → 动画效果 → 启动动画；滑块 3 档 + 档位文案，行内提示实时显示该档组合：性能=`扁平纯色 · 关闭动画 · 跳过启动动画`、均衡=`扁平纯色 · 精简动画 · 播放启动动画`、质量=`流光玻璃 · 完整动画 · 播放启动动画`；点标签可直接跳档                                     |
| P2  | 档位 ↔ 三个底层开关的双向映射（`theme.performanceTier` / `setPerformanceTier`） | stores/theme        | done   | `quality` ⇒ `glass + full + splash on`，`balanced` ⇒ `flat + reduced + splash on`，`performance` ⇒ `flat + off + splash off`；反向派生玻璃优先 —— 手工改材质/动画后滑块位置跟着走（快速设置里改材质同样生效）                                                                                                              |
| P3  | `MSlider` 支持离散档位标签（`labels`）                                          | ui                  | done   | 传 `labels` 时隐藏数字读数、按 `min + i*step` 均分标签、当前档高亮 `text-primary`、点击标签跳档并同时抛 `update:modelValue` + `change`                                                                                                                                                                                     |
| P4  | 「动画效果」分段控件改为便宜档在左：关闭 / 精简 / 完整                          | Settings-appearance | done   | 与性能档位滑块方向一致（越右越贵）；材质行同为 扁平纯色 → 流光玻璃                                                                                                                                                                                                                                                       |
| T1  | 系统托盘：图标 + 菜单（显示主窗口 / 隐藏到托盘 / 退出）                          | Tauri               | done   | 启动即在通知区出现 `MCLR` 托盘（窗口图标），左键单击恢复窗口、右键出菜单；`退出 MCLR` 走 `app.exit(0)`                                                                                                                                                                                                                    |
| T2  | `system.keepInBackground`：关闭窗口时隐藏到托盘而不是退出                        | Core + Tauri        | done   | 默认 `false`（保持旧行为：关闭即退出）；开启后 `CloseRequested` 被 `prevent_close` + `hide`，webview 不重建、任务与游戏进程继续；关闭处理线程直接读 `tray::BackgroundResidency`（`AtomicBool`），`settings_patch` 负责镜像，启动时由已加载设置播种 |
| T3  | 设置入口（设置 → 系统与诊断 → 窗口与后台）                                     | Settings-system     | done   | 「关闭窗口时保持后台运行（系统托盘）」开关，带说明 tooltip；写回 `system.keepInBackground` 并落盘到 `config.json`。先放在「综合设置」页、再挪到「启动」，用户两次都反馈找不到，最终落到侧栏「系统与诊断」页                                                                                                                                                                                                                       |
| T4  | 二次启动从托盘唤回窗口                                                          | Tauri               | done   | single-instance 回调先 `tray::show_main` 再转发 `--launch`：窗口在托盘里时双击启动器图标会重新显示，而不是什么都没发生                                                                                                                                                                                                       |
| T5  | 首次关闭弹窗：隐藏到托盘 / 退出启动器 + 「不再提醒」，选择落到设置里              | Shell + Core        | done   | 关闭按钮先读 `system.closePromptDismissed`：未答过则弹 `MDialog`（含不再提醒勾选框、取消），选「隐藏到托盘」写 `keepInBackground=true`、选「退出启动器」写 `false`，勾选时同时写 `closePromptDismissed=true`；设置 → 系统与诊断 → 窗口与后台「关闭窗口时不再询问」开关可随时重开询问                                |
| R1  | 「检查与修复」不再假成功：等到任务终态再报结果                                  | Launch + Instance   | done   | `install_repair` 只排队任务，原前端拿到 taskId 就弹「完成」；改为 `awaitTask(taskId)`：`failed`/`canceled` 报错误（有 `error` 时用 `formatError`），成功才报「检查与修复完成」；按钮 loading 一直保持到任务结束。实例详情「启动前修复」同改 |
| L1  | 保留天数真正生效：改设置即清理，而不是等下次启动                                | Core + Tauri        | done   | `prune_old_logs` 返回删除数量且始终保留最新一份；`settings_patch` 命中 `/system/logRetentionDays` 时后台立即执行一次清理 |
| L2  | 启动器日志清理命令：立即清理 / 清空 / 目录路径                                  | Tauri               | done   | 新增 `launcher_logs_prune(retentionDays?)`、`launcher_logs_clear()`、`launcher_logs_dir()`，均返回 `{removed, skipped}`；清空保留正在写入的最新文件；Rust 单测覆盖窗口裁剪、清空保留最新、目录不存在 |
| L3  | 日志页操作：打开文件夹 / 清理过期 / 清空（二次确认）                            | Logs                | done   | `日志 → 启动器日志` 的四个动作放在**页面头部 `#actions`**（与「游戏日志」的 复制/导出/清空/分析 同一位置、同一套 tonal/text 语法，刷新用 `MIconButton`），面板内只留文件选择器；删除前走 `confirmDanger`；清理后 `reload` 面板并在当前文件被删时回退到最新一份 |
| L4  | 设置页「立即清理」+ 语义说明                                                    | Settings-system     | done   | 保留天数行加提示「修改后立即清理、最近一份始终保留」与「立即清理」按钮（按当前滑杆值执行）；日志级别行标注「下次启动生效」 |
| N1  | 侧栏补「系统与诊断」入口并拆分原「综合设置」页                                  | Settings            | done   | 原 `misc` 拆成 `network`（网络与下载）与 `system`（系统与诊断：日志级别/保留天数/立即清理/硬件加速）；侧栏「启动器」组新增「系统与诊断」，旧 `?tab=misc` 自动映射到 network |

## 设计说明

- **档位是派生值，不是第四个真相源**：材质存在后端 `ui.material`，动画与启动动画存在
  `localStorage`。再存一份档位就会和这三处不一致；`performanceTier` 用 `computed` 反推
  （玻璃优先，其次看动画预算），所以手工微调后滑块位置自动跟随，旧设置被恢复时也不会失真。
- **滑块不做成不存在的额外能力**：三档只是"一次改三个开关"的便利入口，任何一档都能在下面三行
  里再拆开；因此 P1 保留全部原始控件。
- **关闭即隐藏而非销毁**：`prevent_close` + `hide` 保住同一个 webview，恢复时停留在原页面、
  原滚动位置；销毁再重建会丢掉界面状态。
- **不能 await 设置仓库**：`CloseRequested` 在窗口线程上同步执行，而 `SettingsStore::get()` 是
  异步且有锁；因此用 `AtomicBool` 镜像，`settings_patch` 与启动播种两处写入。
- **托盘常驻**：即使不开后台留存，托盘也提供"隐藏/显示/退出"，让隐藏窗口有回来的路；未开启时
  关闭窗口仍然是退出（不会出现看不见的常驻进程）。
- **「检查与修复」为什么像假功能**：后端 `install_repair` 只创建任务并返回 taskId（真正的校验与
  下载在 tokio 任务里跑），前端把"拿到 taskId"当成"修完了"，于是点击瞬间就弹成功提示，用户看到
  的只有 toast。修复方式复用 v0.0.28 的 `awaitTask`：等 `tasks://update` 的终态，失败/取消报
  错误，成功才报完成；按钮 loading 覆盖整个任务周期。
- **首次关闭为什么问一次而不是直接按设置执行**：默认值是"关闭即退出"，用户可能并不知道托盘
  存在；问一次把两个后果讲清，勾「不再提醒」后才静默执行。设置里给了「关闭窗口时不再询问」
  开关（设置 → 系统与诊断），想重新看到弹窗关掉它即可。

## Release Gate Status

- **State**: Released as `v0.0.29` (版本号已升级, CHANGELOG/PROGRESS 归档, 已打 tag)。
- **Verification**: `node scripts/check.mjs` 全绿（fmt、clippy `-D warnings`、workspace tests、
  bindings 幂等、oxlint/oxfmt、vue-tsc、vitest、i18n、UI contracts、vite build）。
- **Real-app smoke（`MCLR_DATA_DIR` 隔离到临时目录，WebView2 + CDP）**:
  - `/settings/appearance`：性能档位滑块 + 三档标签与提示正确；三行原始控件都在；动画效果顺序
    为 关闭 / 精简 / 完整。
  - 托盘：进程窗口表里出现 `tray_icon_app` 与主窗口；设置 → 启动打开开关后 `config.json`
    写入 `keepInBackground: true`。
  - 关闭按钮：开启时主窗口 `IsWindowVisible=False` 且进程存活（窗口未销毁）；关闭开关后再次
    关闭，进程以 0 退出（旧行为不变）。
  - 托盘唤回：窗口隐藏状态下再启动一次 exe，主窗口 `IsWindowVisible=True`，恢复的是同一个
    窗口实例（`show_main` 日志点命中）。
  - 首次关闭弹窗：设置里关掉「关闭窗口时不再询问」后点 X，弹窗出现且窗口未关闭；「取消」后
    窗口仍在；文案与「不再提醒」勾选框渲染正常。
  - **修复真实做事**：临时 `versions/smoketest` 只留 100 字节的 jar（sha1 与控制端不符），版本
    JSON 指向本地 HTTP 服务；点「检查与修复」后服务端收到 `GET /smoketest.jar`，jar 被补回
    4096 字节并写出 `.mclr-verified.json`，sha1 与控制端一致；点击瞬间无 toast，约 1.4 s 后任务
    终态才出现「检查与修复完成」。删掉 jar 且停掉服务再点，任务 `failed`（bmclapi 404），
    UI 弹出「网络请求失败，请检查网络或代理设置」而不是假成功。
- **日志操作冒烟（`MCLR_DATA_DIR` 隔离目录，WebView2 + CDP）**: 造 9 份历史日志，把设置里的保留天数从 30 拖到 3 后磁盘上立即只剩 3 份（26/27/30）；日志页「清理过期日志」删 4 份并提示「已清理 4 个过期日志」；「清空日志」确认后只剩最新一份，提示「已删除 2 个日志文件」；「打开日志文件夹」无报错。
- **Not measured**: 托盘菜单项的鼠标点击路径（显示/隐藏/退出）未做 UI 自动化，仅验证了同一
  `show_main` / `app.exit(0)` 调用链；托盘图标在高 DPI 缩放下的清晰度未逐档比对；真实整合包
  实例的整包修复耗时未测（冒烟用的是 4 KB 夹具）。

## 假功能 / 未完成 / 缺失 审计（2026-10-30）

审计方式：3 个只读 scout 分片（前端控件、Rust 命令、设置与承诺）+ 主 agent 逐条复核代码。
以下每条都在源码里核实过，不含推测。

### A. 假功能（控件存在，行为与承诺不符）

| ID | 位置 | 现象 | 证据 |
| --- | --- | --- | --- |
| A1 | `src-tauri/src/ipc/tasks.rs` + `crates/mclr-core/src/tasks.rs:191-269` | 任务面板「暂停/继续/取消」不生效：只改 `TaskNode.state`，worker 从不检查；下载继续跑，取消后还会被 `complete_task` 覆盖回 success | 15 处 `create_task` 全部丢弃 token（`_token`/`_cancel_token`）；`mclr-net/downloader.rs` 无 cancel/pause 入参 |
| A2 | `SettingsGamePage.vue` 「游戏启动后启动器动作」 | 值写进 `launch.closeLauncherOnStart`，无任何读取方，启动后窗口既不隐藏也不退出 | 全仓仅 settings 结构/default + 表单引用 |
| A3 | `InstancesToolbar.vue` 排序下拉 + 设置「实例排序准则」 | `sortBy` 只改本地 ref，列表按后端返回顺序渲染；`lastPlayed` 值甚至不在前端联合类型里 | `instances/store.ts:58-92` 只 filter 不 sort；Rust 无 `game.sort_by` 读取 |
| A4 | 语言页「区域格式文化」 | 存 `language.formatCulture`，没有任何格式化消费 | 日期全用 `toLocaleDateString()` 默认区域 |
| A5 | 更新页「启动时自动检测新版本」 | 无启动自检；无更新凭据时开关还是 disabled | 唯一 `update_check` 调用是手动按钮 |
| A6 | 「渲染器」software/d3d12/vulkan | 需要 `<data>/tools/mesa-loader/Loader.jar`，仓库与打包都不带、也没有下载路径，选了只 warn 并回退 | `mclr-game/launch/args.rs:284-297` |
| A7 | 资源「同时禁用/删除依赖」vs「仅处理此模组」 | 两个按钮 emit 同一个 `cascadeConfirm`，依赖项从不处理 | `ResourceModals.vue:128-131`；`ResourceList.vue:376-383` 只处理单条 |
| A8 | 添加离线账号「皮肤模型 Steve/Alex」 | 参数被 `_model` 丢弃，Rust 命令签名里没有 model | `accounts/store.ts:64-73`；`ipc/accounts.rs:45-56` |
| A9 | Authlib 账号「上传皮肤」 | 后端只对 Microsoft 发真实上传；Authlib 仅重裁本地头像就返回，UI 仍 toast「上传成功」 | `ipc/accounts.rs:196-228` |
| A10 | 工具「清理缓存」 | 删除失败被吞，仍返回尺寸，UI 无条件提示已清理 | `ipc/tools.rs:368-380` |
| A11 | 日志页「分析崩溃日志」 | 无选中进程时用硬编码 `'1.20.1-vanilla'` 兜底 | `LogsPage.vue:121` |
| A12 | 公告地址 placeholder「留空使用官方内置公告」 | 留空实际返回空列表，没有内置公告源 | `ipc/home.rs:296-298` |
| A13 | 关于页 FAQ | 声称「点击启动页面自动下载 Java」（0.0.26 已移除）、「微软登录可直接绑定」（仍是敬请期待遮罩） | `settings.json:50,75`；`AccountModals.vue:86-93` |

### B. 未完成

| ID | 位置 | 现状 |
| --- | --- | --- |
| B1 | 界面语言选择 | 只改运行期 `i18n.global.locale`，启动固定 `zh-CN`，重启回退 |
| B2 | 服务器「直连启动」 | 只写 `serverQuickPlay` + 跳转 `/launch`，不真的启动（下一次手动启动才生效） |
| B3 | 微软登录 / 自更新安装 | 后端已实现（device code / updater），缺构建期凭据，UI 分别显示「敬请期待」与占位 |
| B4 | `tools_daily_luck` / `tools_list_media_files` | 命令已注册但没有任何界面调用（后者与已移除的音乐功能同源） |

### C. 该有但没有

| ID | 缺口 |
| --- | --- |
| C1 | 背景音乐：`ui.musicEnabled/musicFolder/musicVolume/musicPlayMode` 四个字段在，UI 与播放器全无 |
| C2 | 字体设置：`ui.fontFamily` 在，无控件也无 CSS 应用 |
| C3 | 下载「自动选择实例」：`download.autoSelectInstance` 在，无 UI、无消费者 |
| C4 | 背景轮播 / 视频：`slideshow`/`video` 渲染链路完整，但没有任何入口能设置（只有 image / none） |
| C5 | 下载断点续传与真正的暂停/取消（与 A1 同根：下载器无取消 API） |
| C6 | 实例排序（与 A3 同根） |

### 复核确认「真实可用」（避免误伤）

工具页（快捷方式 Win32 `.lnk`、皮肤读取/保存/头像、自定义下载、缓存统计）、崩溃分析与导出 zip、首页新闻/公告、启动管线的 RAM/JVM/窗口/标题/IP 栈/优先级/预启动命令/java.exe/GPU 偏好/锁内存/LegacyFix/LWJGL/near-crash 开关、实例重命名/复制/删除/隔离/分组、资源管理全流程、存档与截图、服务器 ping、社区搜索/下载/整合包、日志（含本轮新增清理）、Java 扫描、托盘与关闭弹窗、日志留存。

## 审计整改（同日落地）

| 组 | 处理 |
| --- | --- |
| C1–C4 清理 | 删除 `ui.musicEnabled/musicFolder/musicVolume/musicPlayMode`、`ui.fontFamily`、整块 `DownloadSettings`（含 `download.autoSelectInstance`）；删除背景 `slideshow`/`video` 渲染分支与 `tools_list_media_files`；schema 升到 6，`strip_removed_keys` 清掉旧配置里的这些键（`deny_unknown_fields` 不会因此重置用户设置） |
| A1 | 任务暂停/取消真实生效：`TaskControl`（token + paused 原子）贯穿 `TaskCenter`；下载器在文件之间与分块之间 `checkpoint()`，暂停真正停流量、取消真正停写盘；`complete_task`/`fail_task`/`start_task` 不会复活已取消任务；新增单测 `pause_and_cancel_are_visible_to_workers` |
| A2 | 「游戏启动后启动器动作」落地：`startLaunch` 拿到进程后按设置 hide / `app_quit` |
| A3·C6 | 实例排序实现 name/version/category/lastPlayed；`lastPlayedAt` 由启动流程写入实例配置，工具栏与设置页共用 `settings.game.sortBy` 并双向同步 |
| A4 | `formatCulture` 接入统一 `useFormat().formatDate`（Intl）；启动加载、设置页即时生效，四处重复的本地 formatDate 收敛为一处 |
| A5 | 启动时自动检查更新（仅在有签名更新源时），发现新版本提示一次 |
| A6 | 渲染器：新增 `renderer_agent_available`，缺少 mesa-loader 时只提供「游戏默认」并提示，不再静默回退 |
| A7 | 依赖联动：弹窗拆成 `cascadeConfirmOnly` / `cascadeConfirmAll`，连带禁用/删除真正处理依赖项（批量删除单次确认） |
| A8 | 离线账号皮肤模型贯通：`skinModel` 传进 IPC 与 `AccountStore::add_offline`，不再被丢弃 |
| A9 | Authlib 换肤真实上传（`PUT /api/user/profile/<uuid>/skin`）；不支持的类型明确报错，不再假装成功 |
| A10 | 清理缓存返回真实失败（日志目录保留正在写入的最新文件，其余失败逐条报告） |
| A11 | 崩溃分析不再用硬编码实例 id 兜底，无进程时给出提示 |
| A12·A13 | 公告占位文案改为「留空则不拉取」；FAQ 删除已移除的「自动下载 Java」，微软登录改为如实说明需凭据 |
| B1–B4 | 按用户要求暂不动，记录在 `docs/audit-2026-10-30.md` |
- **A1 实机验证（慢速本地源，20 MB jar）**: 修复任务在 ~1 s 时暂停 → 任务状态 `paused`，目标目录里既没有成品 jar 也没有任何 `.mclr-part` 字节增长（4 s 采样保持 0）；恢复后任务 `success`，jar 20971520 字节、sha1 与源端一致。单测 `paused_task_blocks_before_any_request_and_resumes`（mclr-net）覆盖「暂停时不发请求、不落盘，恢复后完整下载」。
- **迁移验证**: 用带 `schemaVersion:5` + `musicEnabled/musicFolder/musicVolume/musicPlayMode/fontFamily/download.autoSelectInstance` 的旧配置启动，未生成 `config.json.bak`（未回退默认），`ui.seed` 原值保留，落地为 `schemaVersion:6` 且上述键被剥离；`game.sortBy=lastPlayed` 直接生效（列表按最近游玩排序）。

## B 类收尾（同日）

| ID | 处理 |
| --- | --- |
| B1 ✅ | 新增 `src/composables/useLocale.ts`：启动时在首帧前 `ensureLocale()` 读取 `language.locale` 并应用（仅在存在对应语言包时），设置页切换走同一 `setAppLocale`；新增 `useLocale.test.ts` |
| B2 ✅ | 「直连启动」改为真正启动：写直连参数 → 切 `selectedInstanceId` → `startLaunch()` → 「已启动并加入 {name}」；`finally` 恢复上一份 `serverQuickPlay`（一次性语义）；无账号 / 缺 Java 分别提示；`instance-detail` 测试改为断言 `startLaunch` 被调用且配置被还原 |
| B4 ✅ | `tools_daily_luck` 彻底删除：命令 + `DailyLuckResult` + `calculate_daily_luck` + 两条单测 + 前端包装 + mock + 注册项 |
| B3 | 仍未动工（需要发布流程注入微软 / 更新凭据），保留在 `docs/audit-2026-10-30.md` |

---


<!-- ===== v0.0.30.md ===== -->

# MCLR v0.0.30 — 性能与完整性收尾

目标：把扫描、哈希、下载、列表渲染上重复的磁盘读、堆尖峰与串行等待消掉，并顺手修掉审计中
暴露的功能缺陷（版本子目录 mod 不可见、更新未完成就报成功、批量开关逐条 IPC、监听器泄漏）。

| ID  | Slice                                                                       | Owner                        | Status | Acceptance                                                                                                                                                                                                                                             |
| --- | --------------------------------------------------------------------------- | ---------------------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| P1  | 发布 profile：`lto = "thin"` + `codegen-units = 1` + `strip = "debuginfo"`   | Cargo                        | done   | `[profile.release]` 为 workspace 唯一 release profile，无成员 crate 覆盖；不设 `panic = "abort"`（`spawn_blocking` 需能捕获 panic）；`cargo build --release --workspace` 在 MSVC 上成功链接，无 LTO OOM                                             |
| P2  | 前端 vendor 分块                                                             | Vite                         | done   | `@tauri-apps/*` 独立成 `vendor-tauri` chunk（实测 16.70 kB / gzip 4.26 kB）；原 `@iconify-json/*` 规则实测不产生 chunk（UnoCSS 构建期读盘，从不进模块图），已删除并注释原因                                                                              |
| H1  | `hash_file` 零分配流式哈希                                                    | mclr-comp                    | done   | 单个 `File` 两遍：第一遍 SHA1 + 统计非空白字节数，`seek(0)` 后第二遍 MurmurHash2（`h = 1 ^ filtered_len`，4 字节栈上对齐尾缓冲）；堆占用恒为 128 KiB 读缓冲；`test_hash_file_matches_separate_sha1_and_murmur2` 对 300 KiB+、含 `\t\r\n `、filtered 余数 0–3 断言与 slice 实现、reader 实现三方一致           |
| H2  | 短锁 mtime 校验缓存（`localmods.json` / `jarinjar.json`）                      | mclr-comp                    | done   | 镜像为 `cache 路径 -> (mtime, map)`；`fs::metadata`、读盘、serde 全在锁外，锁内只做 `HashMap` 克隆与写入；外部改写或删除缓存文件靠 mtime 不匹配自动失效；`test_cache_isolation_and_toggle_migration` 覆盖迁移、大小不匹配不迁移、外部改写失效 |
| H3  | 冷扫描并行化                                                                  | mclr-comp                    | done   | 未命中条目走 `std::thread::scope`，worker 数 `min(available_parallelism, 4)`；命中条目走原快路径；`save_cache` 只写一次；排序改为 `sort_by_cached_key`                                                                                                       |
| H4  | 切换 mod 的缓存迁移                                                           | mclr-comp                    | done   | `rename` 后按目标文件真实 `size` / `mtime_secs` 迁移缓存条目，重命名后的 jar 不再被重新哈希；大小不匹配则放弃迁移（回落到重新扫描）                                                                                                                        |
| D1  | 下载写缓冲与校验缓冲                                                          | mclr-net                     | done   | `chunk.downloaded` 只在 `write_all` 成功后递增，`chunk.current_offset()` 恒等于磁盘上已写字节；`SlowChunk` 切分前、`checkpoint()` 前、流结束前均 flush；`verify_file` 读缓冲 64 → 256 KiB；`test_slow_chunk_splitting`、`test_range_206_multichunk_download`、`paused_task_blocks_before_any_request_and_resumes` 全绿 |
| D2  | 整合包解包与实例重扫移出异步线程                                              | mclr-comp                    | done   | `extract_modpack_overrides` 与收尾 `scan_instances` 走 `spawn_blocking`，`JoinError` 映射为 `CompError::Io` / `GameError::Io`                                                                                                                            |
| D3  | 实例扫描父版本 JSON 记忆化                                                    | mclr-game                    | done   | `scan_instances` 内 `parent_cache` 命中即复用，同一父版本只读解析一次；缺失/损坏的父版本同样被记忆，避免重复尝试                                                                                                                                          |
| U1  | 启动器日志统一虚拟化                                                          | Logs                         | done   | 唯一 `MVirtualList :threshold="200"`：≤200 行渲染纯 DOM，>200 行只渲染可视行；`[data-testid="launcher-log-content"]` 保留；新增两条测试（250 行虚拟化、2 行纯 DOM）                                                                                       |
| U2  | 任务批量更新只发布一次                                                        | Tasks store                  | done   | `handleTaskUpdate` 克隆一次、按 `Map<id, index>` 应用整批、末尾单次赋值；`taskTree` / `activeTasks` / 聚合速度每批只算一次                                                                                                                                |
| U3  | 启动页监听器注销                                                              | Launch                       | done   | `onUnmounted` 调用 `launchStore.stopListening()`                                                                                                                                                                                                          |
| U4  | 大实例分组跳过 FLIP                                                           | Instances                    | done   | `<TransitionGroup :name="group.items.length > 24 ? undefined : 'm-list'">`                                                                                                                                                                               |
| F1  | 版本子目录 mod 可见性                                                         | Tauri + mclr-comp            | done   | `resources_list` 先 `scan_instances` 取 `mc_version`，把 `mods/<ver>/` 一并扫描并入；元数据先按完整路径匹配、再回退 `(file_name, size)`；`scan_resources` 按归一化 `rel_path` 去重；`test_scan_resources_includes_legacy_mc_version_subfolder` 覆盖                                                                     |
| F2  | 批量更新并发 + 单文件原子替换                                                 | Tauri                        | done   | `buffer_unordered(net.max_concurrent_files.clamp(1,16))`；每个 jar 校验通过后立刻删除被替换的旧 jar；每个文件开始前 `checkpoint()` 一次（取消即返回 `NetError::Canceled`）；进度按完成数原子累加                                                              |
| F3  | 更新结果等待任务终态                                                          | Resources                    | done   | `doSingleUpdate` / `doBatchUpdate` 先 `awaitTask(taskId)`；`failed` / `canceled` 弹 `toast.danger`（有 error 用 `formatError`，否则 `resources.update.failed`），成功才刷新列表                                                                              |
| F4  | 重复项判定收紧                                                                | Resources                    | done   | `resourceDuplicateKey` 只返回 `mod:<id>` / `proj:<id>`，两者皆无返回 `null`；`duplicateStats` 单次遍历生成 keys + 计数，`filterCounts` 与 `duplicate` 过滤器共用                                                                                            |
| F5  | `resources://patch` O(P)                                                      | Resources                    | done   | 每个事件批建一次 `Map<fileName, entry>`，逐条按 key 应用；不加 `requestAnimationFrame`，托盘隐藏时同样即时生效                                                                                                                                            |
| F6  | 批量开关 8 并发                                                               | Resources                    | done   | `runToggleBatches` 按 `TOGGLE_CONCURRENCY = 8` 切片 `Promise.all`；`doToggle` 自身吞异常，单个失败不中断整批                                                                                                                                               |
| F7  | 排序绑定当前语言                                                              | Resources                    | done   | `computed(() => new Intl.Collator(locale.value, { numeric: true, sensitivity: 'base' }))`，替换 `String#localeCompare`，语言切换后自动重排                                                                                                                |
| F8  | 截图列表去掉不可靠的目录 mtime 缓存                                            | mclr-saves                   | done   | 实测本机 NTFS 对目录时间戳是惰性更新（200 次「写文件后立即 stat 父目录」有 125 次 mtime 未变），因此新增截图会被旧缓存挡住、删除的截图仍显示；改为每次重扫，并把逐文件 `path.is_file()` + `fs::metadata(&path)` 换成 `DirEntry::file_type()` / `entry.metadata()`；1000 张截图的分页调用实测 196 ms → **3.0 ms**，`test_paged_listing_is_stable_across_calls` 连续 25 次全绿（改前 15 次循环里必现失败）                                                                              |
| F9  | mod 扫描去掉逐文件重复 stat                                                    | mclr-comp                    | done   | `scan_local_mods` 用枚举自带的 `file_type` / `metadata`，不再 `path.is_file()` 后再 `fs::metadata(&path)`（Windows 上每次都是一次独立 stat，实测 1000 文件 96 ms → 0.7 ms）                                                                                     |

## 设计说明

- **两遍流式而不是「先过滤再算」**：MurmurHash2 的种子是 `1 ^ filtered_len`，长度必须在流结束
  后才知道。把过滤后的字节攒进 `Vec::with_capacity(file_len)` 会按文件大小分配内存——100 MB jar
  并发四个就是数百 MB 尖峰。第一遍已经读过的页留在 OS page cache 里，第二遍顺序重读几乎无成本，
  代价只有一次 128 KiB 缓冲。
- **锁只保护内存，不保护 IO**：旧实现每次 `scan_local_mods` 都持全局 mutex 读文件并 JSON 解析，
  三个页面同时扫描时会互相排队。现在 mtime 在锁外取，命中就锁内克隆 map，未命中就锁外读盘/解析
  后锁内落盘，外部进程改写缓存文件也会因为 mtime 变化自动失效。
- **`chunk.downloaded` 是磁盘真值，不是内存真值**：分块下载会把慢速分块按
  `current_offset() + rem/2` 切开重试。若先递增计数再写盘，切分点就会落在磁盘数据之外，续传会
  产生错位。因此计数只在 `write_all` 成功后推进，切分/暂停/取消前一律先 flush。
- **每文件原子替换，而不是整批结束再删**：批量更新 20 个 mod，若最后一个下载失败，旧实现要么
  留下新旧两份 jar，要么把已更新的旧 jar 也删掉。改成「自己的新 jar 校验通过就删自己的旧 jar」，
  失败只影响它自己。
- **重复项必须有身份**：`title || fileName` 兜底会把两个不同 mod 的同名 jar 判成重复，用户按
  「重复」筛选后看到的是假警告。只有 `modId` / `projectId` 才能证明「这是同一个项目」。
- **虚拟化用组件内阈值，而不是两套模板**：`MVirtualList` 自带 `threshold`，≤200 行直接渲染纯
  DOM（保留文本选择与 `data-testid`），超过才挂虚拟滚动。两套 DOM 分支会让测试和样式都要维护两份。
- **托盘隐藏时也要更新**：`resources://patch` 的批处理已经由 Rust 侧按 50 ms / 64 条合并，前端
  再加 `requestAnimationFrame` 会在窗口不可见时停摆，因此只用同步 `Map`。

## Release Gate Status

- **State**: 已发布为 `v0.0.30`（版本号已升级，CHANGELOG / PROGRESS 归档，已打 tag，已产出 NSIS 安装包）。
- **Verification**: `node scripts/check.mjs` 全绿 —— `cargo fmt --all --check`、
  `cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`（205 passed /
  4 ignored，ignored 均为需要真实网络的 live 测试）、`pnpm gen:bindings` 幂等、`oxfmt --check`、
  `oxlint`、`vue-tsc --noEmit`、`vitest run`、`lint-i18n`、`lint-ui-contracts`、`vite build`。
- **Release build**: `cargo build --release --workspace` 5m00s 成功（thin LTO + CGU=1，无 MSVC
  链接 OOM）；`mclr.exe` 37.53 MiB，无 `.debug*` 段（`.text` 27.83 MiB、`.rdata` 8.46 MiB）。
- **Package**: `pnpm tauri build` 4m03s 完成，产出
  `target/release/bundle/nsis/MCLR_0.0.30_x64-setup.exe`（22.99 MiB / 24,110,208 字节，
  `FileVersion` / `ProductVersion` 均为 `0.0.30`）；`bundle.targets = ["nsis"]`，
  `installMode = currentUser`，WebView2 走 `downloadBootstrapper`。
- **Packaged-binary smoke**: 用 `MCLR_DATA_DIR` 隔离到临时目录启动 release 二进制，12 秒后进程
  存活、窗口标题 `MCLR`，并生成 `.minecraft/versions`、`launcher_profiles.json`、`cache/news_page_1.json`
  （14,464 字节，说明网络与缓存链路通）、`master.key`、当日日志文件；随后结束进程并清理临时目录。
- **测试增量**: Rust 新增 `test_hash_file_matches_separate_sha1_and_murmur2`、
  `test_cache_isolation_and_toggle_migration`、
  `test_scan_resources_includes_legacy_mc_version_subfolder`；前端新增日志虚拟化 2 条、
  重复项判定 1 条、patch 批处理 1 条、更新等待/失败 3 条、批量开关并发 1 条。
- **未覆盖**: 无 CI 环境下的 live 网络测试（curseforge/modrinth `live_*`，仍为 `ignored`）；
  本次未做基准压测，性能数字为机制推导。

---


<!-- ===== v0.0.31.md ===== -->

# MCLR v0.0.31 — 玻璃材质修复、相邻页预取、动态社区筛选与整合包命名后置

目标：修掉打包后玻璃材质整层失效的生产 bug（LightningCSS 丢弃 `backdrop-filter`），把官方新闻与
社区资源的翻页从「每次等网络」变成「预取 + 命中即渲染」，让社区筛选随 tab 与平台变化而收窄，
并把整合包的实例命名从「挡住版本列表的前置输入」改成「点击安装后的确认」。

| ID  | Slice                                                                              | Owner              | Status | Acceptance                                                                                                                                                                                                                                 |
| --- | ---------------------------------------------------------------------------------- | ------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| G1  | `-webkit-backdrop-filter` 前置                                                       | Tauri (前端主题)    | done   | `material.css` 内 11 处同名规则把 `-webkit-backdrop-filter` 写在标准属性之前；生产 CSS 中 `.m-panel` / `.m-card` / `.m-overlay` / `.m-drawer` / `.m-nav-pill` / `.m-titlebar-scrolled` 均同时保留 `-webkit-` 与标准 `backdrop-filter:blur(...)` |
| G2  | 浮层统一到 `.m-overlay.m-refract`                                                     | UI                  | done   | `MPagination`、`MTooltip` 加 `m-refract`；`MDialog` 去掉覆盖 token 的 `bg-panel` / `border-line`；新闻跳页按钮由 `bg-panel/90 backdrop-blur-md` 改为 `m-overlay m-refract`，与旁边的分页器同材质 |
| G3  | `.m-titlebar-scrolled` 表面规则                                                       | Tauri (前端主题)    | done   | 平铺：canvas 底 + `border-line` + `shadow-1`；玻璃：`rgb(255 255 255 / 0.58)` + `blur(22px)`；暗色玻璃：`rgb(24 27 34 / 0.65)`；`TitleBar` 滚动态只切类名，删除随之失效的 `useThemeStore` |
| P1  | 新闻页缓存、请求去重与相邻页预取                                                       | Home store         | done   | `newsPageCache` / `inFlightNews` 以页码为键；`loadNews(1)` 后自动预取第 2 页，`loadNews(2)` 命中缓存直接渲染且**只请求过一次**（`news-p2` 内容直出，`newsFetch` 第 2 页调用计数恒为 1）                                                       |
| C1  | CurseForge 分类与加载器语义                                                            | mclr-comp          | done   | `resolve_cf_category_id` 按 tab 作用域映射（同一 `adventure` 键在 mod=422 / datapack=6948 / world=248），数字 id 透传、未知键返回 `None`；`modLoaderType` 仅在 `Mod` / `Modpack` 下发送，wiremock 用 `query_param_is_missing` 验证光影请求不带该参数 |
| C2  | 筛选契约集中到 `comp/constants.ts`                                                     | Comp frontend      | done   | `COMP_CATEGORIES_BY_TYPE` 覆盖六个 tab（`cfId` / `mrSlug` 缺省即代表该平台不支持）；`getLoaderDefsForType`、`isPlatformSupportedForType`、`isSortSupportedOnPlatform`、`isCategorySupportedOnPlatform`、`resolveCategoryForPlatform` 单一来源 |
| C3  | 每页 24 条                                                                           | Comp store         | done   | `limit = 24`（可被 1/2/3 列网格整除，`all` 平台拆 12 + 12）；mock 兜底同步为 24                                                                                                                                                              |
| C4  | 上下文筛选清洗                                                                        | Comp store         | done   | `sanitizeFiltersForContext` 在每次 `search` 前执行：`world + modrinth` 落到 CurseForge，CurseForge 下 `follows` 回落 `relevance`，当前平台无任何可用加载器时清空 loader，平台不支持的分类清空                                                       |
| C5  | 结果页缓存、请求去重与相邻页预取                                                        | Comp store         | done   | 缓存以完整 filterKey 为作用域，filter 变更或强制刷新即整体失效；`all` 平台双端各取半页，单平台独占组合（world、CF 专有分类、光影加载器）改满页单端查询；`page ± 1` 后台预取，翻页命中缓存不再请求 |
| U1  | 工具栏动态选项与禁用态                                                                 | Comp frontend      | done   | 分类随 tab 变化，平台不支持的分类 `disabled`；加载器按 tab 决定显隐（资源包/数据包/世界隐藏），`shader + curseforge` 禁用选择器而 `mod + curseforge` 保持可用；`MC 版本` 从「目标实例」容器中拆出，在 modpack tab 也可见 |
| U2  | 分页 dock 让位                                                                        | Comp frontend      | done   | 结果容器 `pb-20`，浮动分页器不再压住最后一行卡片                                                                                                                                                                                            |
| M1  | 整合包实例命名后置                                                                    | Comp frontend      | done   | 快速下载弹窗不再预置名称输入；点「开始安装」才弹出 `MDialog`（标题预填并按 `[/\\?%*:\|"<>]` 清洗），未确认前 `modpackInstallRemote` 调用次数为 0；关闭外层弹窗会清理内层状态；modpack 的加载器 chips 恢复可用 |
| I1  | i18n 补齐                                                                             | Comp locales       | done   | 新增 83 个 `comp.search.categories.*` / `comp.search.loaders.vanilla` 键；`constants.ts` 中全部 `labelKey` 均可解析，仅 `all` 由页面提供                                                                                                      |

## 设计说明

- **属性顺序不是风格问题**：Vite 8 的 LightningCSS 在同一规则里看到 `-webkit-backdrop-filter`
  排在标准属性之后时，会把标准 `backdrop-filter` 当作重复声明丢掉，而 Chromium 126+ / WebView2
  只认标准属性——结果是开发环境有玻璃、打包后整层透明。把前缀放前面，两者都能留下。
- **`.m-refract` 只做增强**：`url(#mclr-refract)` 在部分内核里整条声明无效，因此它永远与
  `.m-overlay` 同挂，模糊由 `.m-overlay` 保证，折射有内核支持时才叠上去。
- **缓存以 filterKey 为作用域，而不是用一个「脏」标记**：翻页远比改筛选频繁，用完整筛选串做键
  可以让「同一筛选下第 N 页」成为一个可缓存的纯值；改筛选直接换键，无需手工失效，也不会串页。
- **相邻页预取必须静默**：预取失败不能变成用户可见的错误，也不能挡住当前页渲染，因此失败只
  吞掉、下次真正需要时再请求。
- **`all` 平台不是「平均分」**：两端都支持时各取半页交错合并，行高才稳定；只有一端支持当前
  组合时改满页单端查询，否则会出现「半页空白」的假象。
- **命名必须在「选中某个版本」之后**：整合包没有目标实例，它的名字是安装产物的一部分；放在
  版本列表之前等于强迫用户先想名字再看版本。
- **禁用而不是隐藏**：平台/加载器不被支持时保留选项并置灰，用户能看出「这里被限制了」，而不是
  以为功能消失。

## Release Gate Status

- **State**: 已发布为 `v0.0.31`（版本号已升级，CHANGELOG / PROGRESS 归档，已打 tag，已产出 NSIS 安装包）。
- **Verification**: `node scripts/check.mjs` 全绿 —— `cargo fmt --all --check`、
  `cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`（207 passed /
  4 ignored，ignored 均为需要真实网络的 live 测试）、`pnpm gen:bindings` 幂等、`oxfmt --check`、
  `oxlint`、`vue-tsc --noEmit`、`vitest run`（298 passed / 32 files）、`lint-i18n`、
  `lint-ui-contracts`、`vite build`。
- **生产 CSS 复核**: `dist/assets/index-*.css` 中 `.m-panel` / `.m-card` / `.m-overlay` /
  `.m-drawer` / `.m-nav-pill` / `.m-titlebar-scrolled` 的玻璃规则均同时保留
  `-webkit-backdrop-filter:blur(...)` 与标准 `backdrop-filter:blur(...)`（改前标准属性被
  LightningCSS 丢弃，打包后 `backdropFilter` 实测为 `none`）。
- **Package**: `pnpm tauri build` 7m27s（release profile，thin LTO + CGU=1），产出
  `target/release/bundle/nsis/MCLR_0.0.31_x64-setup.exe`（24,114,702 字节 / 23.00 MiB，
  `FileVersion` / `ProductVersion` 均为 `0.0.31`）；`bundle.targets = ["nsis"]`，
  `installMode = currentUser`，WebView2 走 `downloadBootstrapper`。
- **Packaged-binary smoke**: 用 `MCLR_DATA_DIR` 隔离到临时目录启动 release 二进制，14 秒后进程
  存活、窗口标题 `MCLR`，生成 `.minecraft/versions`、`launcher_profiles.json`、`master.key`、
  当日日志，以及 `cache/news_page_1.json`（14,464 字节）与 **`cache/news_page_2.json`
  （12,147 字节）**——后者直接证明相邻页预取在打包产物里生效。
- **测试增量**: Rust 新增 `test_resolve_cf_category_id_maps_tab_scoped_keys`、
  `test_curseforge_search_category_and_shader_loader_filtering`（wiremock 断言 `categoryId`
  存在且 `modLoaderType` 缺失）；前端新增社区分页预取与缓存 1 条、tab/平台筛选联动 1 条、
  整合包命名后置 1 条、新闻分页预取 1 条。
- **未覆盖**: 玻璃折射（`url(#mclr-refract)`）依赖内核支持，只在具备该能力的 WebView2 版本上
  呈现，自动化只验证标准 `backdrop-filter` 未被丢弃；live 网络测试仍为 `ignored`。

---

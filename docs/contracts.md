# Cross-Cutting Contracts

This document is the definitive engineering specification for runtime invariants, IPC security, error semantics, UI behavior, and quality verification across MCLR.

---

## 1. Paths & Filesystem (`mclr_core::paths`)

- **Root Data Directory**: `data_dir() = %APPDATA%/MCLR` (overridable via `MCLR_DATA_DIR` during automated testing).
- **Standard Children**:
  - `config.json`: Application settings.
  - `accounts.enc`: AES-256-GCM encrypted account database.
  - `master.key`: DPAPI-protected 32-byte master encryption key.
  - `cache/`: Download chunks, UVMC manifests, asset caches.
  - `logs/`: Daily rolling launcher logs (`mclr.YYYY-MM-DD.log`).
  - `runtime/`: Managed Java runtime installations.
  - `tools/`: Downloaded utility helpers (e.g. `authlib-injector.jar`).
  - `.minecraft/`: Default game folder (when selected by user).

---

## 2. Configuration & State Management (`mclr_core::settings`)

### 2.1 Settings Store
- **Unified Schema**: `Settings` struct with `#[serde(rename_all="camelCase", default)]` and `schemaVersion: u32 = 2` (v1 configurations migrate automatically on load).
- **Persistence**: Atomic write (write to `.tmp` then rename), debounced 300 ms flush.
- **IPC Interface**:
  - `settings_get() -> Settings`
  - `settings_patch(patch: serde_json::Value)`: RFC 7386 JSON Merge Patch; deserializes into `Settings` to validate, rejects unknown fields with `AppError { code: "settings.invalid" }`.
  - **Broadcast**: Emits on Tauri channel `settings://changed` when configuration mutates.
- **Modularity**: Sub-settings (`launch`, `java`, `net`, `ui`, `update`, `language`) reside in dedicated files under `mclr-core/src/settings/`.

### 2.2 System Tray & Background Residency (`src-tauri/src/tray.rs`)
- **Tray Menu**: Built once during `setup` with window icon; actions: `显示主窗口` / `隐藏到托盘` / `退出 MCLR`. Left click restores window, right click opens context menu.
- **Background Residency**: `system.keepInBackground` (default `false`) controls window close behavior:
  - When `true`: `WindowEvent::CloseRequested` for the main window is intercepted and hides the window so downloads/tasks continue running.
  - When `false`: Closing the window terminates the application cleanly.
- **Thread Safety**: `settings_patch` mirrors the residency flag into atomic state (`tray::BackgroundResidency`), preventing synchronous window-thread blocking.
- **Single-Instance Wake**: Subsequent launches forward CLI arguments via `tauri-plugin-single-instance` and invoke `tray::show_main` to restore hidden windows.
- **First-Close Prompt**: The initial close prompts user preference with a 「不再提醒」 checkbox, storing choices in `system.keepInBackground` and `system.closePromptDismissed`.

### 2.3 Launcher Logging & Rotation (`mclr_core::logging`)
- **Log Files**: Stored in `paths::logs_dir()` as `mclr.YYYY-MM-DD.log` (`tracing_appender` daily rotation).
- **File Retention**: Guarded by `system.logRetentionDays` (default 7 days). The newest log is always preserved as it is held open by the active appender.
- **Pruning & Maintenance**:
  - `launcher_logs_prune(retentionDays?)`: Prunes logs exceeding retention threshold.
  - `launcher_logs_clear()`: Purges all historical logs except the currently active file.
  - Setting mutations on `system.logRetentionDays` trigger immediate background pruning.
  - `system.logLevel` is baked into `EnvFilter` at boot; modifications take effect on next launch.

---

## 3. Error Handling Architecture

- **Backend Discipline**: Every backend crate maintains a `thiserror` enum implementing `fn code(&self) -> &'static str` (`<crate>.<kind>`, e.g. `net.timeout`, `auth.xsts.noXboxAccount`) and an `Into<AppError>` conversion.
- **Structured Representation**: `AppError` serializes to:
  ```json
  { "code": "crate.kind", "params": { "key": "value" }, "detail": "Optional technical context" }
  ```
- **Frontend Localization**: The webview never parses raw English error strings. The UI maps `error.<code>` from `src/locales/zh-CN/error.json` via `useErrorText().formatError(err)`.

---

## 4. Background Tasks & Progress Windows (`mclr_core::tasks`)

- **Task Hierarchy**: `TaskCenter` manages an in-memory tree of active and completed tasks:
  ```rust
  pub struct TaskNode {
      pub id: String,
      pub parent_id: Option<String>,
      pub title_key: String,
      pub title_params: HashMap<String, String>,
      pub state: TaskState, // waiting | running | paused | success | canceled | failed
      pub progress: f32,    // 0.0 ..= 1.0
      pub speed_bps: u64,
      pub error: Option<AppError>,
  }
  ```
- **Progress Windows**: Multi-stage tasks utilize `TaskCenter::set_progress_window(id, base, span)` to map independent 0.0..=1.0 stage reports into monotonic slices (e.g. Modpack installation: 8% archive -> 62% base/loader -> 28% files -> 2% overrides). Progress never rewinds.
- **Task Lifecycle**: Controlled via `TaskControl` checkpoints. Tasks pause and cancel cooperatively at safe boundaries.
- **Task Cleansing**: `TaskCenter::clear_finished()` removes terminal tasks (`success`, `failed`, `canceled`). A finished child is kept while its parent task is still active.
- **IPC Interface**: `tasks_list`, `tasks_clear_finished`, `task_cancel(id)`, `task_pause(id)`, `task_resume(id)`. Broadcasts stream on Tauri channel `tasks://update`.

---

## 5. IPC Protocol & Security Containment

### 5.1 Protocol Conventions
- **Command Naming**: Commands follow `<feature>_<verb>` snake_case (e.g. `instances_list`, `modpack_import_file`).
- **Data Naming**: JSON payloads use `camelCase` across frontend and backend boundaries (`#[serde(rename_all="camelCase")]`).
- **Channel Events**: Tauri events follow `<feature>://<name>` (e.g. `tasks://update`, `launch://logBatch`, `accounts://deviceCode`).
- **Non-Blocking Execution**: Any synchronous disk I/O, archive compression, cryptographic hashing, NBT parsing, or image manipulation **must** run inside `tokio::task::spawn_blocking` or `mclr_app::ipc::run_blocking`.
- **Command Registry**: All commands are explicitly registered in `src-tauri/src/commands.rs` within marked `// [feature]` blocks.

### 5.2 Path Containment (IPC Validation)
All path-taking IPC commands validate targets in `src-tauri/src/security.rs` prior to execution:

| Tier | Guarded Commands | Constraint |
|---|---|---|
| `ensure_allowed` / `ensure_all_allowed` | `resources_delete`, `resource_toggle`, `resources_import`, `screenshot_*`, `tools_save_skin`, `tools_custom_download`, `tools_cleanup_failed_tasks` | Target path must canonicalize strictly inside the launcher data directory, cache/logs/runtime/tools directories, or configured game instances |
| `ensure_not_system_path` | `modpack_inspect`, `schematic_inspect`, `tools_generate_skin_avatar`, `path_open`, `assets_allow` | User-picked files may reside anywhere on user storage, but system roots, `\Windows`, `Program Files`, and `ProgramData` are refused |

*Note*: Both tiers resolve the nearest existing ancestor, neutralizing `..` traversal, junctions, symlinks, and Windows verbatim (`\\?\`) bypasses.

### 5.3 Content Security Policy (CSP)
Enforced in `src-tauri/tauri.conf.json` and verified by `src-tauri/tests/csp_test.rs`:
- `script-src 'self'`: Zero `'unsafe-eval'` or `'unsafe-inline'` in production.
- `connect-src 'self' ipc: http://ipc.localhost`: **The webview never connects to the internet directly**; all network traffic routes through Rust.
- `img-src`: Whitelists remote asset hosts (Modrinth, ForgeCDN, Minecraft/Microsoft news domains), `data:` (skins/SVGs), and `asset:` (local screenshots/backgrounds). No blanket `https:`.
- `object-src 'none'`, `base-uri 'self'`, `frame-ancestors 'none'`, `form-action 'none'`.
- Local filesystem access through `asset:` is restricted to runtime-authorized scopes (`ipc::assets::allow_defaults`).

---

## 6. Internationalization & Locale Discipline (i18n)

- **Engine**: `vue-i18n` v11 in Composition mode (`legacy: false`), default `locale` and `fallbackLocale` = `zh-CN`.
- **Key Scheme**: Dot-separated hierarchy `area.sub.element` in camelCase (e.g. `accounts.dialog.msDeviceTitle`).
- **Bundle Aggregation**: Locale files reside in `src/locales/<locale>/<feature>.json` and auto-merge via `import.meta.glob('./locales/*/*.json', { eager: true })`.
- **Formatting Rules**: Plurals use `vue-i18n` pipe syntax. Numbers, byte sizes, and dates utilize deterministic app formatters (`formatBytes`, `formatDate`) bound to `language.formatCulture`.
- **Lint Gate**: `scripts/lint-i18n.mjs` enforces:
  - Zero raw CJK characters in `src/**/*.{vue,ts}` outside `src/locales/`, `*.test.ts`, and `mock.ts`.
  - Every static `t('...')` reference must resolve to an existing key in `zh-CN`.
  - Dynamic runtime keys (`t('error.' + code)`, `t(task.titleKey)`) must exist in their respective domain locale tables.

---

## 7. High-Risk Secondary Confirmation Contract

Destructive or irreversible operations mandate explicit secondary confirmation via `useDialog().confirmDanger(...)`:

1. **Covered Operations**:
   - Deleting a Minecraft instance (purges files and configuration).
   - Deleting a world save (`saves.deleteConfirm`).
   - Removing an account from the local vault.
   - Deleting a Java runtime configuration.
   - Deleting local mods, resource packs, shaders, or server entries.
   - Cleaning cache categories in Toolbox (permanently deletes cache or log files).
   - Clearing active game process log buffers.
   - Canceling active background tasks or downloading jobs.
   - Force-terminating / killing a running Minecraft process.
2. **Visual Contract**:
   - Severity badge with warning/danger icon (`i-lucide-triangle-alert` in `bg-danger-soft text-danger-fg`).
   - Detailed callout well (`.m-well`) highlighting the exact item title, file path, or size to be purged.
   - Confirm action button styled with `variant="danger"`.
3. **Acknowledgment Checkbox Guard (`requireCheckbox`)**:
   - Extreme-risk operations (such as world deletion) require checking an explicit acknowledgment checkbox before the confirm button enables.

---

## 8. UI Component & Motion Contract

Every component in `src/ui/` and all feature views must satisfy seven core design dimensions:

1. **Light Mode**: High-contrast OKLCH tonal palette ($>= 4.5:1$ text contrast vs canvas/raised, $>= 3:1$ borders/controls).
2. **Dark Mode**: Low-glare dark OKLCH tonal palette ($>= 4.5:1$ text contrast, $>= 3:1$ interactive elements).
3. **Flat Material**: Crisp, opaque surfaces (`bg-canvas`, `bg-raised`, `bg-sunken`, `bg-overlay`) without backdrop blur overhead.
4. **Liquid Glass Material**: Acrylic/Mica translucency, frosted backdrop blur, specular inner border highlights, and `.m-refract` SVG displacement filter.
5. **Skeleton State**: Components must provide an inline skeleton placeholder (via `MSkeleton` or `:loading="true"` / `:skeletonLines="N"`) matching the exact border-radius and bounding box of real content to prevent Cumulative Layout Shift (CLS).
6. **Loading State**: Non-blocking loading indicators on buttons (`:loading="true"`), inputs, and lists without freezing user navigation.
7. **Motion Tiers (`html[data-motion='full' | 'reduced' | 'off']`)**:
   - `full`: Complete spring and cubic-bezier transitions, transforms, staggered entrances, and displacement animations.
   - `reduced`: Disables translate, scale, and stagger transforms; disables skeleton shimmer; transforms rotating spinners into subtle opacity pulses (`m-pulse-reduced`).
   - `off`: Sets all animation and transition durations to `0s !important`; replaces animations with static indicators.

---

## 9. Performance Invariants & Resource Guarantees

Load-bearing performance invariants guarded by automated regression tests:

| Area | Invariant | Guard Test |
|---|---|---|
| Mod Hashing (`mclr_comp::local`) | `hash_file` streams two passes over one `File` handle with a single 128 KiB buffer (pass 1: `Sha1`, pass 2: `murmur2_stream_with_len`). Never buffers the filtered file in memory. | `test_hash_file_matches_separate_sha1_and_murmur2` |
| Mod Cache (`localmods.json`) | Mutex guards only in-memory `HashMap` clone/insert. Parsing, hashing, and serialization happen outside the lock. Keyed by path and invalidated on mtime mismatch. | `test_cache_isolation_and_toggle_migration` |
| Cache Miss Concurrency | Cold mod scans fan out over `std::thread::scope` bounded to `min(available_parallelism, 4)` workers to cap buffer allocations. | `test_scan_local_mods_streamed_hashes_match_pinned_values` |
| Chunked Downloader (`mclr_net`) | `chunk.downloaded` advances strictly after `write_all` flushes to disk. Slowness never exhausts error retries; slow chunks split or resume without failing. | `test_slow_chunk_splitting`, `test_slow_chunk_never_fails_the_download` |
| Blocking Work | ZIP extraction, directory rescans, hashing, and instance scans never run on async runtime threads; they dispatch via `spawn_blocking`. | Verified by architecture rules |
| Directory Scans | Inside `read_dir`, inspect `DirEntry::file_type()` / `metadata()`. Never call `path.is_file()` or `fs::metadata(&path)` redundantly (measured: 96 ms vs 0.7 ms over 1000 files). | Verified by architecture rules |
| Safe Scroll Containers | Page and tab containers use `overflow-x: clip` rather than `hidden` to prevent CSS computed `overflow-y: auto` from spawning nested fake scrollports. | Verified in `material.css` & `InstanceDetailPage.vue` |
| Long Text Virtualization | Any unbounded list or log view renders via `MVirtualList` with built-in thresholding. | Verified by log and mod tests |
| Store Event Coalescing | High-frequency events (`tasks://update`) are batched before dispatching to frontend stores, triggering a single reactive recomputation per batch. | Verified by task benchmarks |
| Event Lifecycle | Every event listener registered with `listen()` must unlisten on component unmount (`onBeforeUnmount` / `onUnmounted`). | Enforced across all feature views |

---

## 10. Engineering Discipline & Verification Gate (`pnpm check`)

Code quality across backend and frontend is strictly enforced with zero allowable compiler warnings and zero warning suppressions:

### 10.1 Zero-Suppression Policy
- **Rust Backend**:
  - `cargo clippy --workspace --all-targets -- -D warnings` must pass cleanly.
  - Zero `#[allow(...)]` attributes across all production crates.
  - Functions with >7 parameters must encapsulate arguments into structured parameter objects.
- **Frontend / TypeScript**:
  - Zero `eslint-disable`, `@ts-ignore`, or `@ts-expect-error` directives in production code.
  - Zero raw `v-html` directives; all Markdown and rich HTML must be sanitized through `<MMarkdown>`.
  - Icon names in templates must be static string literals (`i-lucide-*`, `i-simple-icons-*`) for UnoCSS build extraction.

### 10.2 Verification Gate Sequence
Every commit must pass the unified verification pipeline:
```bash
# Executed from app/
pnpm check
```
The gate executes the following sequence:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `pnpm gen:bindings` (re-exports `src/ipc/bindings.ts` and asserts idempotence)
5. `pnpm exec oxfmt --check src`
6. `pnpm exec oxlint`
7. `vue-tsc --noEmit`
8. `vitest run`
9. `node scripts/lint-i18n.mjs`
10. `node scripts/lint-ui-contracts.mjs`
11. `vite build`

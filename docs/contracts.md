# Cross-Cutting Contracts

## 1. Paths & Filesystem (`mclr_core::paths`)
- Root data directory: `data_dir() = %APPDATA%/MCLR` (override via environment variable `MCLR_DATA_DIR` during testing).
- Standard children:
  - `config.json`: Application settings.
  - `accounts.enc`: AES-256-GCM encrypted account database.
  - `master.key`: DPAPI-protected 32-byte encryption key.
  - `cache/`: Download chunks, UVMC manifests, asset caches.
  - `logs/`: Daily rolling launcher logs (`mclr.YYYY-MM-DD.log`).
  - `runtime/`: Managed Java installations.
  - `tools/`: Downloaded utility helpers (e.g. `authlib-injector.jar`).
  - `.minecraft/`: Default game folder (if user selects it).

---

## 2. Settings Store (`mclr_core::settings`)
- Unified `Settings` struct: `#[serde(rename_all="camelCase", default)]` with `schemaVersion: u32 = 2` (v1 configs are migrated on load: recommended JVM arguments + GPU preference).
- Persistence: Atomic write (write to `.tmp` then rename), debounced 300 ms save.
- IPC API:
  - `settings_get() -> Settings`
  - `settings_patch(patch: serde_json::Value)`: RFC 7386 JSON Merge Patch; deserializes into `Settings` to validate, rejects invalid fields with `AppError{code: "settings.invalid"}`.
  - Event: Broadcasts on `settings://changed` channel when configuration mutates.
- Modularity: Feature sub-settings (`launch`, `java`, `net`, `ui`, `update`, `language`) reside in dedicated files under `mclr-core/src/settings/` and are registered in `settings/mod.rs`.

---

## 2b. System Tray & Background Residency (`src-tauri/src/tray.rs`)
- The tray icon (`main-tray`) is built once in `setup` with the window icon; menu: 显示主窗口 / 隐藏到托盘 / 退出 MCLR, left click restores the window, right click opens the menu.
- `system.keepInBackground` (default `false`) controls the window close button: when on, `WindowEvent::CloseRequested` for the `main` window is prevented and the window is hidden, so downloads/tasks keep running; when off, closing exits the app as before.
- The close handler runs on the window thread and must not await the settings store, so `settings_patch` mirrors the flag into `tray::BackgroundResidency` (an `AtomicBool`); startup seeds it from the loaded settings.
- A second launch (single-instance plugin) calls `tray::show_main` before forwarding `--launch`, so a launcher that is sitting in the tray comes back instead of staying invisible.
- The first close asks the user (hide to tray vs. quit) with a 「不再提醒」 checkbox: the picked action is written to `system.keepInBackground`, and `system.closePromptDismissed` records whether the question should be skipped next time (both editable in 设置 → 系统与诊断 → 窗口与后台).

---

## 2c. Launcher Logs (`mclr_core::logging` + `src-tauri/src/ipc/update.rs`)
- Files live in `paths::logs_dir()` as `mclr.<YYYY-MM-DD>.log`, one per day (`tracing_appender` daily rotation).
- Retention is a *file* window (`system.logRetentionDays`, default 7): the newest file is always kept, because it is the one the running appender writes to.
- `launcher_logs_prune(retentionDays?)` applies the window on demand (default = stored setting); `launcher_logs_clear()` deletes every launcher log except the newest; `launcher_logs_dir()` returns the directory so 「打开日志文件夹」 can reuse `path_open`. Both cleanup commands return `{ removed, skipped }`.
- A `settings_patch` that touches `system.logRetentionDays` prunes immediately in the background, so lowering the window takes effect without restarting.
- `system.logLevel` is baked into the `EnvFilter` at startup: changing it still requires a restart (the settings row says so).

---

## 3. Error Handling
- Each backend crate maintains a `thiserror` enum implementing `fn code(&self) -> &'static str` (`<crate>.<kind>`, e.g. `net.timeout`, `auth.xsts.noXboxAccount`) and an `Into<AppError>` conversion.
- IPC handlers return `Result<T, AppError>`. `AppError` serializes to `{ "code": "...", "params": { ... }, "detail": "..." }`; the UI renders `error.<code>` from `src/locales/zh-CN/error.json` via `useErrorText()`.
- Frontend maps error codes to localized strings in `locales/*/error.json`.

---

## 4. Background Tasks (`mclr_core::tasks`)
- `TaskCenter` holds an in-memory tree of active and completed tasks:
  ```rust
  pub struct Task {
      pub id: String,
      pub parent_id: Option<String>,
      pub title_key: String,
      pub title_params: HashMap<String, String>,
      pub state: TaskState, // waiting | running | success | canceled | failed
      pub progress: f32,    // 0.0 ..= 1.0
      pub speed_bps: Option<u64>,
      pub error: Option<AppError>,
  }
  ```
- Cancellation & pause managed via `tokio_util::sync::CancellationToken`.
- Event streaming: Coalesced updates broadcast on Tauri channel `tasks://update`.
- IPC API: `tasks_list`, `task_cancel(id)`, `task_pause(id)`, `task_resume(id)`.
- All long-running operations (game installation, resource download, modpack import/export, self-updates) must register as background tasks with localized `task.*` title keys.

---

## 5. IPC Naming & Handler Conventions
- **Command Naming**: Commands follow `<feature>_<verb>` snake_case (e.g. `instances_list`, `install_version_compat`).
- **Payload Naming**: JSON fields are camelCase on both frontend and backend (`#[serde(rename_all="camelCase")]`).
- **Event Naming**: Channel events follow `<feature>://<name>` (e.g. `launch://logBatch`, `tasks://update`).
- **Non-Blocking Rule**: Any `#[tauri::command]` performing synchronous disk I/O, archive compression, cryptographic hashing, NBT parsing, or image manipulation **must** run inside `mclr_app::ipc::run_blocking(...)`.
- **Registry**: Commands are registered in `src-tauri/src/commands.rs` inside marked `// [feature]` blocks.

---

## 6. High-Risk Secondary Confirmation Contract
Destructive or irreversible operations must mandate explicit secondary confirmation via `useDialog().confirmDanger(...)`:

1. **Covered Operations**:
   - Deleting a Minecraft instance (removes files and game records).
   - Deleting a world save (`saves.deleteConfirm`).
   - Removing an account from the local vault.
   - Deleting a Java runtime configuration.
   - Deleting local mods, resource packs, shaders, or server entries.
   - Cleaning cache categories in Toolbox (permanently deletes HTTP, log, or skin caches).
   - Clearing active game process log buffers.
   - Canceling active background tasks or downloading jobs.
   - Force-terminating / killing a running Minecraft process.
2. **Visual Contract**:
   - Must render a prominent severity warning/danger icon badge (`i-lucide-triangle-alert` in `bg-danger-soft text-danger-fg`).
   - Detailed callout box (`.m-well`) highlighting the exact item title, file path, or size to be purged.
   - Confirm action button must use `variant="danger"`.
3. **Acknowledgment Checkbox Guard (`requireCheckbox`)**:
   - For high-severity data loss operations (such as permanently deleting a world save), an acknowledgment checkbox (e.g. `saves.deleteConfirm.checkbox`) must be specified. The confirm button remains disabled until the user checks this acknowledgment.

---

## 7. 7-Dimension UI Component Contract
Every component in `src/ui/` and every feature surface must satisfy seven design dimensions verified across test and lint suites:

1. **Light Mode**: High-contrast OKLCH tonal palette ($\ge 4.5:1$ text contrast vs canvas/raised, $\ge 3:1$ borders/controls).
2. **Dark Mode**: Low-glare dark OKLCH tonal palette ($\ge 4.5:1$ text contrast, $\ge 3:1$ interactive elements).
3. **Flat Material**: Crisp, opaque, or subtle tinted surfaces (`bg-canvas`, `bg-raised`, `bg-sunken`, `bg-overlay`) without backdrop blur overhead.
4. **Liquid Glass Material**: Acrylic/Mica translucency, frosted backdrop blur, specular inner border highlights, and `.m-refract` SVG displacement filter.
5. **Skeleton State**: Components must provide an inline skeleton placeholder (via `MSkeleton` or `:loading="true"` / `:skeletonLines="N"`) matching the exact border-radius and bounding box of real content to prevent Cumulative Layout Shift (CLS).
6. **Loading State**: Non-blocking loading indicators on buttons (`:loading="true"`), inputs, and lists without freezing user navigation.
7. **Motion Tiers (`html[data-motion='full' | 'reduced' | 'off']`)**:
   - `full`: Complete spring and cubic-bezier transitions, transforms, staggered entrances, and displacement animations.
   - `reduced`: Disables translate, scale, and stagger transforms; disables skeleton shimmer; transforms rotating spinners into subtle opacity pulses (`m-pulse-reduced`).
   - `off`: Sets all animation and transition durations to `0s !important`; replaces animations with static indicators.

---

## 8. Zero-Warning & Zero-Suppression Policy
Code quality across backend and frontend is strictly enforced with zero allowable warnings and zero warning suppressions:

- **Rust Backend**:
  - `cargo clippy --workspace --all-targets -- -D warnings` must pass cleanly.
  - Zero `#[allow(...)]` attributes across all crates (including `dead_code`, `clippy::too_many_arguments`, and `async_fn_in_trait`).
  - Functions with >7 parameters must encapsulate arguments into structured parameter objects (e.g. `ModpackExportParams`, `LaunchPipelineParams`, `ModpackImportContext`).
- **Frontend / TypeScript / Vue**:
  - Zero `eslint-disable`, `@ts-ignore`, or `@ts-expect-error` directives allowed in production sources.
  - Zero unvetted `v-html` directives; all Markdown and rich HTML must be rendered through `<MMarkdown>` (which applies DOMPurify sanitization).
  - All template icon names must be static string literals (`i-lucide-*`, `i-simple-icons-*`) so UnoCSS can extract them during build.

---

## 9. Automated Verification Gate (`pnpm check`)
Every change must pass the unified check gate before merge:
```bash
# Executed from app/
pnpm check
```
The gate executes the following sequence:
1. `cargo fmt --all --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `pnpm gen:bindings && git diff --exit-code src/ipc/bindings.ts`
5. `pnpm exec oxfmt --check src`
6. `pnpm exec oxlint`
7. `vue-tsc --noEmit`
8. `vitest run`
9. `node scripts/lint-i18n.mjs`
10. `node scripts/lint-ui-contracts.mjs`
11. `vite build`

## Content Security Policy

`src-tauri/tauri.conf.json → app.security.csp` is enforced in the webview (`devCsp` is the looser dev variant: Vite HMR needs `ws://localhost:*` and inline bootstrap scripts).

Design rules, guarded by `src-tauri/tests/csp_test.rs`:

- `script-src 'self'` in production — no `'unsafe-eval'`, no `'unsafe-inline'`.
- `connect-src 'self' ipc: http://ipc.localhost` — **the webview never talks to the internet**; every request goes through Rust. An injected script cannot exfiltrate or fetch arbitrary URLs.
- `img-src` enumerates every remote host the UI renders (Modrinth / ForgeCDN / the Minecraft+Microsoft news domains whitelisted in `ipc/home.rs`), plus `data:` (skin heads, SVG noise) and `asset:` (local screenshots, backgrounds). Never a blanket `https:`.
- `object-src 'none'`, `base-uri 'self'`, `frame-ancestors 'none'`, `form-action 'none'`.
- Local files are exposed through the `asset:` protocol only for paths granted at runtime (`ipc::assets::allow_defaults` → data dir + configured game folders); the static scope list stays empty.
- Supported hardening headers are set via `app.security.headers` (`X-Content-Type-Options: nosniff`, `Cross-Origin-Opener-Policy: same-origin`, a camera/mic/geolocation-denying `Permissions-Policy`). `Referrer-Policy` is not in Tauri's allow-list.

## Path containment (IPC)

Tauri v2 capabilities only restrict **plugin** commands; the app's own `#[tauri::command]`s are callable by anything inside the webview. Every command that accepts a path therefore validates it in `src-tauri/src/security.rs`:

| Tier | Used by | Rule |
|---|---|---|
| `ensure_allowed` / `ensure_all_allowed` | `resources_delete`, `resource_toggle`, `resources_import`, `screenshot_*`, `tools_save_skin`, `tools_custom_download` | Must resolve inside the launcher data dir, cache/logs/runtime/tools dirs, or a configured game folder |
| `ensure_not_system_path` | `modpack_inspect`, `schematic_inspect`, `tools_generate_skin_avatar`, `path_open`, `assets_allow` | Dialog-picked files may live anywhere the user owns, but filesystem roots, `\Windows`, `Program Files`, `ProgramData\Microsoft` etc. are refused |

Both tiers canonicalize the nearest existing ancestor, so `..` segments, junctions and Windows verbatim (`\\?\`) prefixes cannot be used to slip past `starts_with`.

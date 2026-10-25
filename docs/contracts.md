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
- Unified `Settings` struct: `#[serde(rename_all="camelCase", default)]` with `schemaVersion: u32 = 1`.
- Persistence: Atomic write (write to `.tmp` then rename), debounced 300 ms save.
- IPC API:
  - `settings_get() -> Settings`
  - `settings_patch(patch: serde_json::Value)`: RFC 7386 JSON Merge Patch; deserializes into `Settings` to validate, rejects invalid fields with `AppError{code: "settings.invalid"}`.
  - Event: Broadcasts on `settings://changed` channel when configuration mutates.
- Modularity: Feature sub-settings (`launch`, `java`, `net`, `ui`, `update`, `language`) reside in dedicated files under `mclr-core/src/settings/` and are registered in `settings/mod.rs`.

---

## 3. Error Handling
- Each backend crate maintains a `thiserror` enum implementing `fn code(&self) -> &'static str` (`<crate>.<kind>`, e.g. `net.timeout`, `auth.xsts.noXboxAccount`) and an `Into<AppError>` conversion.
- IPC handlers return `Result<T, AppError>`. `AppError` serializes to `{ "code": "...", "message": "...", "details": ... }`.
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

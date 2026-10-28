# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.27] - 2026-10-28

Performance + resource iteration. No feature was added to the game pipeline; the
work removed per-frame and per-call work while keeping the UI pixel-identical.

### Added
- **Settings → 赞助支持**: WeChat / Alipay QR codes served from `public/sponsor/`
  (static files, so `img-src 'self'` already covers them), placed last in the
  settings sub-navigation.
- **Explicit placeholders instead of failures**: 检查更新 and the About page's
  repository / feedback actions render a disabled control with a 「敬请期待」
  badge when the build has no signed update feed, and never call the command.

### Changed
- **Logs panel is O(new lines) instead of O(10 000)**: `shallowRef` array with a
  monotonic `seq`, incremental filtering while the filter key is unchanged, a
  binary search trim at the cap, and one `scrollHeight` read per flush. Measured
  per-flush cost of the append path: 8.67 ms → 0.034 ms (254×).
- **Skeleton shimmer runs on the compositor**: the `background-position`
  keyframes became a `translateX` layer whose geometry is exactly equivalent
  (verified across 13 phases × light/dark; worst phase mean 0.256/255).
- **Fewer forced layouts and repaints**: sidebar pill geometry updates at most
  once per animation frame, the ambient drift pauses while scrolling, theme
  writes coalesce into a single frame with a memoized palette, the task progress
  tween follows a target instead of restarting per event, and favourite lookups
  plus version classification are memoized.
- **`transition-all` narrowed** to the properties that actually change (progress
  fill, card, sidebar pill).
- **Startup repair is skipped when nothing changed**: a `.mclr-verified.json`
  fingerprint (version JSON mtime/length + asset index id) short-circuits
  `repair_instance` on the launch path (`RepairMode::Fast`); 检查与修复 always
  runs the full pass.
- **Download engine**: bounded worker pool instead of one task per file,
  event-driven progress instead of an O(n) polling sweep, no `Range` probe when
  the size is known, and hashes computed while streaming instead of re-reading
  finished files.
- **Mod metadata**: Modrinth lookups batched per 100 sha1, one metadata cache
  (`localmods.json` no longer written/read), jars parsed and hashed straight from
  disk instead of being read into memory.
- **Cheaper background work**: `java_scan` returns the cache without rescanning,
  instance sorting precomputes its keys instead of doing IO in the comparator,
  screenshot pages are cached by directory mtime, the launcher log is read from
  the tail only, settings reads hand out `Arc` snapshots instead of deep clones,
  blocking IO moved into `spawn_blocking` (asset index, spec verification, loader
  installers), and the instance scanner holds its lock only around the cache.
- Save icons are no longer base64-inlined into `SaveInfo`.

### Fixed
- Concurrent first-run agent extraction truncated a shared temp file, making
  `agents_apply_to_instances_that_inherit_their_parent_json` flaky (~1 in 5 runs).
- `{detail}` placeholders in error copy rendered literally because the frontend
  never forwarded `AppError.detail` into the interpolation parameters.

### Removed
- **Project license**: `LICENSE`, every Cargo `license` field, the
  `package.json` entry and the About badge. No license is granted at this time;
  third-party components keep their own terms in `THIRD_PARTY_NOTICES.md`.
- **GitHub-dependent update wiring**: the release-feed fallback, repository /
  issue-tracker links and the startup auto-check are gone until a repository
  exists; the signed in-app update path is untouched.

## [0.0.26] - 2026-10-28

### Added
- **Launch-time agents (PCL parity)**: bundled `lwjgl-unsafe-agent.jar` (injected only for LWJGL 3.4.1 / MC 26.x) and `legacyfix.jar` (injected only when `releaseTime < 2013-06-25`), written atomically to `<data>/tools/agents/` on first use, each switchable in 高级启动选项 and disabled by default-off semantics that never break launch when extraction fails. Inherits-from chains (NeoForge 26.x, Forge 1.5.2) are resolved before either rule is evaluated.
- **Java management without a global default**: `java.select` is gone (settings schema v5). Every launch derives the runtime from the version's own `javaVersion` requirement and picks the installed Java closest to it — exact major first, then the nearest major above; anything below the requirement is never used. `26.3` and the `26.x` snapshot scheme map to Java 25.
- **Per-instance Java override** (实例详情 → 设置) is honoured only when it can actually run that version; otherwise a compatible install is used and the fallback is logged.
- **Fast, actionable missing-Java failure**: launch aborts before repair work, the banner names the required major and offers a one-click jump to Java management.
- **Production CSP** (`script-src 'self'`, `connect-src` limited to the IPC bridge, remote images enumerated per real source) with `nosniff`/COOP/permissions headers and five drift-guard tests.
- **IPC path whitelist** (`src-tauri/src/security.rs`): writes/deletes confined to the launcher directory, reads/opens reject system directories.
- **Material-aware selected states**: the sidebar active pill is now styled per material — solid tonal block in `flat`, translucent accent wash with blur, inner highlight and glow in `glass` (`.m-nav-pill` / `.m-nav-accent`).
- **Desktop hardening in release builds**: native context menu, reload and devtools shortcuts are blocked; `devtools: false`.

### Changed
- Launch switches are positive (`开 = 启用`) and every default-on item now starts enabled; schema v4 migration inverts the old `disable*` keys without losing intent.
- Java selection prefers precision over novelty: a Java 17 instance no longer receives a Java 25 runtime, and the chosen runtime is logged (`Launching instance … with Java 25.0.3 …`).
- The Java cache is rescanned when a cached list cannot satisfy a requirement, so a runtime installed after the last scan is found on the next attempt.
- Java page header now matches the instances/accounts pattern (primary action + ⟳ icon button); the JDK preference moved next to the installed-runtime list.
- Resource toolbar hover menus became `MPopover` instances, gaining collision handling, click and keyboard access.

### Fixed
- **MC 26.x failed to start with `UnsupportedClassVersionError`**: the version scheme (`26.3`, `26.4-snapshot-6`) fell through the legacy Java branches, and a pinned global Java overrode the real requirement. Both paths are gone; verified live on 26.3 (Java 25) and 1.21.1 NeoForge (Java 21).
- **Instance context menu never appeared** (wrong prop watched) and was dismissed by the very right-click that opened it (window `contextmenu` listener); it now closes on outside pointer-down or `Esc` only.
- **Instance card top edge clipped on hover** inside the collapse animation container.
- **Path traversal**: modpack `files[].path`, Forge `maven/` zip entries and `remove_mod_files` are now validated through `safe_join` (absolute, drive-letter and `..` paths rejected, with unit tests).
- **Data loss risks**: Create failure no longer deletes an existing instance directory, stale `.mclr-installing` markers expire after six hours, corrupt `config.json` is backed up as `config.json.bak`, and short native writes clean up their partial files.
- **Silent failures**: LabyMod install without a chosen build no longer installs vanilla; `modify` honours `removeModFiles`; `resources://patchBatch` event name matches the emitter; OptiFine install errors surface instead of being swallowed; duplicated log lines removed.
- `cleanup_stale` no longer removes live files; auto memory stays below the safety cap; crash-report tokens are redacted in snake_case too.
- 48 backend error codes and 9 task titles gained copy (83/83 backend codes covered); 15 invalid lucide icon names rendered blank and were replaced.

### Removed
- **Java runtime auto-download**: `java_download`, the Mojang runtime manifest resolver and the banner's download button are gone. MCLR only picks among installed runtimes and points at Java management.
- Global "default Java" (`java.select`) and the automatic pinning of a lone detected runtime.

## [0.0.25] - 2026-10-28

### Added
- **Advanced Launch Options Panel** (`设置 → 启动`): renderer bridge (software / DX12 / Vulkan), JVM argument head with recommended defaults and one-click reset, game argument tail, global pre-launch command with `{java}`/`{minecraft}`/`{verpath}`/`{username}` placeholders, high-performance GPU preference, `java.exe` console mode, locked heap (`-Xms = -Xmx`) and crash-analysis toggle — every row carries a hint tooltip and every option is actually applied during launch.
- **Settings schema v2 migration**: existing profiles get the recommended JVM arguments and the GPU preference default without losing custom values; clearing a field afterwards is respected.
- **Official loader artwork everywhere**: `MLoaderIcon` now serves the bundled brand logos (Fabric, Forge, NeoForge, Quilt, LabyMod, OptiFine, Cleanroom, plus the grass block for vanilla) on light hue-tinted chips, at a legible size; `MSelect` gained a square `image` option so the community-resource loader filter shows the same artwork.
- **Skin avatar generator** (tools): PCL-compatible face upscaling with hat overlay, exportable at 64/128/256/512.
- **Java default selection**: a single detected runtime becomes the default automatically, with explicit set/clear actions.
- **PCL-style mod selection**: click anywhere on a card to select, toolbar select-all / clear, left accent bar instead of checkboxes.
- **Shared floating-position utility** (`src/ui/floating.ts`): flip + shift + size clamping for viewport-fixed layers.
- **Microsoft login placeholder**: the add-account entry is now masked with a "敬请期待" badge instead of starting an unfinished OAuth flow.

### Changed
- Installer loader section reworked: equal-width cards that fill the row, inline compact version picker, incompatible extras hidden instead of disabled.
- Instance diff preview no longer reserves a block; it lives behind the hint icon next to the configuration summary.
- `MHelpTooltip` reuses one implementation for every hint (icon configurable, default question mark).

### Fixed
- **Last-used instance lost on restart**: the self-healing watcher cleared the stored id before the instance list had loaded.
- **Loader version identification** rewritten to PCL rules: arguments first (`--fml.neoForgeVersion` / `--fml.forgeVersion`), per-loader Maven parsing (FancyModLoader version is no longer mistaken for the NeoForge version, `-beta`/`-universal` classifiers preserved correctly) and one entry per loader — validated across 1.12.2 / 1.20.1 / 1.20.2+ / 26.3.
- **LabyMod auto naming** never applied; switching back to vanilla also kept the stale suffix.
- **Duplicate instance name** showed the raw i18n key (`instance.name.duplicate`); errors now render through the shared error formatter and the collision is detected instantly.
- **Clipped floating layers**: dropdowns, tooltips, popovers and context menus could overflow the window or their panel; they now flip/shift within the viewport and re-anchor on scroll.
- **Narrow-window deformations**: launch-page selectors overflowed their half-width cards, settings text inputs pushed past the card edge, and `MSegmented` labels painted outside the panel — the control now collapses to icons only when the labels genuinely do not fit.
- **15 invalid lucide icon names** (`help-circle`, `alert-circle`, `check-circle`, `trash-2`, `edit-2`, …) rendered nothing; all icon references are validated against the icon set again.
- **Tab/doc focus rings clipped** by `overflow-hidden` panels (log file picker, installer cards) — space is reserved instead of clipping.
- Resource toolbar hover menus could overflow the window; they now align to the right edge with a max width.

## [0.0.24] - 2026-10-26

### Added
- **Concurrent Mod Update Checks (10x-20x Speedup)**:
  - Re-architected `check_local_mods_updates` in Rust backend (`crates/mclr-comp::local`):
    - Concurrent Modrinth SHA1 and version update queries via `tokio::join!`.
    - Concurrent CurseForge mod files query pool powered by `tokio::task::JoinSet` with shared client connection pooling.
  - Added dedicated "检查更新" (Check Updates) action in `ResourceToolbar.vue` with instant single-spinner animation and feedback toasts.
- **Resource Item Spacing & Breathing Room**: Added explicit `:gap="10"` spacing between mod, resource pack, and shader cards in `MVirtualList`, eliminating dense unspaced rows.

### Fixed
- **Sidebar Active Pill Drift on UI Scale**:
  - Bound `themeStore.scale` watcher in `SidebarNav.vue` and attached a persistent `ResizeObserver` to the navigation container, ensuring the active pill remains 100% pinned (`delta: 0px`) under any scale ratio (0.8x to 1.25x).
  - Synchronized container scroll listener with pill positioning.
- **Instance Detail Page Horizontal Overflow Elimination**:
  - Added `overflow-x-hidden` to outer `<main>` in `AppShell.vue` and sub-tab containers in `InstanceDetailPage.vue`, completely eliminating unwanted page-wide horizontal scrollbars.
- **Modify Instance Tab Bottom Truncation**:
  - Removed harmful `-mx-4 -mb-4` negative margins on the bottom sticky action bar in `ConfiguratorInstanceStep.vue`, ensuring configuration summary, reset, and modify actions are 100% visible with full padding.
  - Added bottom padding `pb-10` in `InstallTab.vue`.
- **Side Drawer Glass Material Refinement**:
  - Replaced distorted 8px SVG refraction filter on `MDrawer.vue` with dedicated `.m-drawer` high-opacity frosted glass (`0.88` alpha with `28px` heavy blur and elevated shadows in dark and light modes), eliminating see-through transparency.
- **Duplicate Spinner on Check Updates Button**: Unified `MButton` loading state to display exactly one clean spinning circle.

## [0.0.23] - 2026-10-26

### Added
- **Universal Image Component (`MImage`)**: Introduced a standardized image container with built-in loading fade-in transitions, automatic fallback to `fallbackSrc` or `fallbackIcon` upon network/404 errors, pixelated texture support (`:pixelated="true"`), and full `loading="lazy"` / `decoding="async"` contract compliance. Replaced all 17 raw `<img>` instances across the entire application.
- **Authentic Minecraft & Loader Icon Assets**:
  - Imported official high-resolution pixel art textures from PCL-CE (`Fabric`, `NeoForge`, `Forge`, `Quilt`, `OptiFine`, `LabyMod`, `Cleanroom`, `Grass`, `GrassPath`, `CobbleStone`, `GoldBlock`, `RedstoneBlock`, `CommandBlock`, `Egg`, `RedstoneLampOn`, `RedstoneLampOff`) to `public/assets/icons/`.
  - Upgraded `icons.ts` and `MLoaderIcon.vue` to support authentic pixelated image textures and official vector brand logos.
- **PCL-Aligned Instance Detail Navigation (Two-Tier Grouping)**:
  - Replaced overcrowded 11-tab horizontal bar with a two-tier logical grouping via `MSegmented` and filtered `MTabs`:
    - 🎮 **Game Management (`game`)**: Overview (`overview`), Auto-Install (`install`), Settings (`settings`), Export (`export`).
    - 📦 **Resource Management (`resources`)**: Mods (`mods`), Resource Packs (`resourcepacks`), Shaders (`shaders`), Saves (`saves`), Screenshots (`screenshots`), Schematics (`schematics`), Servers (`servers`).
  - Retained 100% backward compatibility with `?tab=...` route query params.
- **PCL-Aligned Mod Management Operations (`ResourceList`)**:
  - Added instant batch toggle actions (Enable All / Disable All).
  - Restructured `ResourceItemRow` with compact layout, quick-toggle switch for `.jar` / `.jar.disabled`, and More menu (Reveal in Explorer, Jar-in-Jar inspector, Delete).

### Changed
- **Solid Flat Material & Zero Bleed-Through**:
  - Restricted ambient background blobs in `AmbientBackdrop.vue` strictly to `material === 'glass'`, providing a clean, solid, opaque canvas in `flat` material.
  - Window TitleBar when scrolled in `flat` mode uses solid `bg-canvas border-b border-line shadow-sm` without translucent text bleed.
- **Light & Dark Mode Tonal Layers Overhaul**:
  - Redesigned light mode palette (`canvas`: 0.965, `panel`: 0.985, pure white `raised` cards, crisp `line`: 0.885 and `lineStrong`: 0.785) with multi-layered Slate-900 shadows (`15 23 42`), passing all 17 WCAG AA/AAA contrast targets.
  - Re-anchored dark mode layer hierarchy (`canvas`: 0.145, `panel`: 0.180, `raised`: 0.220, `sunken`: 0.125).
- **Unified Page Header & Prominent Title Presence (`MPageHeader`)**:
  - Enlarged title icon tile to `w-12 h-12 rounded-xl` with `w-6 h-6` icon, and title typography to `text-3xl font-extrabold tracking-tight`.
  - Added `#icon` and `#badge` slot support.
  - Reduced `.m-page` top padding from `1.5rem` to `1rem` to eliminate awkward empty voids at the top of the screen.
- **Compact Instance Cards & Zero Redundant Void**:
  - Reduced `InstanceCard.vue` height from `h-36` (144px) to `h-28 min-h-[112px]`, eliminating 32px of empty vertical dead space.
- **Download Center Compact Padding**:
  - Rebuilt `VersionCategoryTabs.vue` using Settings-About-Help style compact accordion cards (`padding="none"`, `px-4 py-2.5` header, `px-4 py-2` version items, 46px virtual row height).
  - Compacted configuration wizard steps and removed bloated paddings across pages.
- **Filtered Non-Authentic Minecraft Icons**:
  - Purged 11 generic Lucide icon placeholders from `minecraftIconList`, strictly exposing authentic Minecraft block textures in icon pickers.

### Fixed
- **Double Divider Line in Instance Detail**: Eliminated redundant outer `border-b border-line pb-1` container surrounding `MTabs` in `InstanceDetailPage.vue`.

## [0.0.21] - 2026-10-26

### Added
- **Unified Global Overlay Architecture (`MOverlay` / `MDrawer` / `MDialog`)**: Built standard overlay primitive supporting stacked Escape key dismissal, body scroll-lock, and liquid glass backdrops. Added `MDrawer` supporting right/left/bottom sliding panels with smooth spring animations.
- **High-Risk Secondary Confirmation (`useDialog().confirmDanger`)**: Introduced semantic danger badges, warning callout cards, and mandatory acknowledgment checkboxes across all destructive operations (game kill, cache cleanup, log clearing, task cancellation, Java removal, instance deletion, and world save deletion).
- **PCL Parity Community Resources Download Flow**:
  - Clicking any resource card now opens an interactive **Quick Download Dialog** (`CompQuickDownloadDialog`) with instance selector, PCL canonical version chips, mod loader chips, direct version file list, and instant download actions.
  - Added PCL-aligned actions matrix in resource detail: official project home (`项目主页`), MCMod百科 search (`MC 百科`), copy name, copy link, and favorite toggle.
  - Modpacks now follow dedicated new instance creation flow prompting for new instance name and directly importing via `modpackInstallRemote`.
  - Filtered target instances to modable instances only (`i.modable`) on the Mod tab.
- **PCL Loader & Version Compatibility Matrix**:
  - Implemented 1.20.1 NeoForge special compatibility rule: NeoForge 1.20.1 instances automatically recognize and support 1.20.1 Forge mods.
  - Implemented Quilt -> Fabric, Cleanroom 1.12.2 -> Forge 1.12.2, and LegacyFabric -> Fabric loader compatibility.
  - Restored global floating paginator (`MPagination`) on Community Resources.
- **Shared UI Primitives**: Extracted standardized `MSearchBar` with clear button and debounced queries, and semantic `MStatusBadge` with pulse indicators.
- **Rust Backend Decomposition & Pure Calculation Offload**: Decomposed oversized modules into submodules (`mclr-game::launch`, `instance`, `install`, `mclr-saves::saves`, `mclr-comp::modpack`), offloaded version compatibility engine into `mclr_game::compat`.

### Fixed
- **Mod Toggle Self-Healing**: Fixed bug where disabling a mod prevented it from being re-enabled due to stale filename paths. Added bidirectional `.disabled` resolution in Rust backend and reactive path synchronization in frontend.
- **Daily Rolling Launcher Log Suffix**: Configured daily rolling appender with `mclr.YYYY-MM-DD.log` suffix and unified `is_launcher_log_file` matcher handling both legacy and modern log files across pruning, Toolbox cache calculation, and Logs page.
- **Logs Page Fixed Viewport & Inner Scrolling**: Fixed outer page height on `/logs` to eliminate double scrollbars and nested clipping, wrapping inner log view in `rounded-xl overflow-hidden`.
- **Comp Versions Accordion & Padding Overhaul**: Restored local responsive accordion state in `CompVersionsTab` and removed quadruple-nested card padding.
- **Skeleton Shimmer Dimensions**: Fixed `MSkeleton` default inline styles stomping over flex utilities and causing crushed vertical lines.
- **Vite File Watcher `EBUSY`**: Excluded `**/target/**` and `**/crates/**` from Chokidar file watcher to eliminate Windows build locks on `mclr.exe` and `mclr.pdb`.
- **Zero Warnings & Zero Suppressions**: Completely removed all `#[allow(...)]` attributes across Rust crates and all `@ts-ignore` / `eslint-disable` comments across frontend.
- **Theme Seed Palette Differentiation**: Fixed low-chroma clamp causing `slate` and `sky` to generate identical blue themes, giving `slate` an elegant neutral cool gray tone and `sky` vivid cyan-blue. Localized live preview components.
- **Removed Folder Path from Launch Instance Card**: Cleaned up subtitle path display in `InstancePreviewCard` to match Instance Management card standards.

## [0.0.20] - 2026-10-26

### Added
- **Fast Toolchain Integration (Oxc)**: Integrated Rust-powered `oxlint` and `oxfmt` with dedicated `.oxlintrc.json` and `.oxfmtrc.json` configs, adding `pnpm lint` and `pnpm fmt` scripts directly to the gate pipeline.
- **Offline Account Default Skin Customization**: Added default skin model picker (`Steve (经典 4px)` / `Alex (纤细 3px)`) in offline account creation dialog with full persistence.
- **Liquid Glass TitleBar on Scroll**: Window TitleBar dynamically transitions into a liquid glass frosted acrylic material with backdrop blur when the page scrolls down.
- **Enhanced NeoForge Loader Detection**: Added support for 1.20.1 `FancyModLoader` coordinates (`net.neoforged.fancymodloader:loader:<ver>`), `--fml.neoForgeVersion` arguments, and unique loader family deduplication to eliminate multi-badge explosion.
- **Rich Account Select Dropdown**: Home page account selector now renders avatars, localized mode badges (`[微软]`, `[离线]`, `[第三方]`), clean name-adjacent badge alignment, and far-right checkmarks.

### Changed
- **Simplified Instance Cards**: Removed redundant `Minecraft <version>` badge (displayed directly beneath instance title), eliminated bottom divider and folder path, and compacted card height to `h-36`.
- **Removed Vanilla Rubbish Demotion**: Standard pure Vanilla instances now remain permanently classified as "常规" (`Regular`) without artificial per-drop degradation.
- **Eliminated Horizontal Layout Shift Without Blank Gutters**: Anchored page left margin to window width (`100vw`) and sidebar width CSS variable `--sidebar-w`, achieving rock-solid zero-pixel shift when switching between long and short tabs with no right blank gutter.
- **MSelect Scrollbar Track Padding**: Added 6px vertical track margins to dropdown listbox scrollbars, preventing scrollbar thumb clipping by rounded container corners.
- **PCL Canonical Version Filter in Comp Search**: Standardized community platform search versions to PCL-CE canonical release list with descending sort.
- **Material Text Refinement**: Renamed "液态玻璃" to "玻璃" in QuickSettings, and removed parentheses descriptions (`(高性能)` / `(液态亚克力/云母效果)`) in Appearance settings.

## [0.0.19] - 2026-10-25

### Added
- **Unified Task Management in Logs (`/logs?tab=tasks`)**: Integrated tasks list into Logs page as a 3rd tab, retiring standalone `/tasks` route and sidebar entry while linking titlebar TaskIndicator directly to `/logs?tab=tasks`.
- **Java Runtime Management Parity**: Added OS file manager path opening (`path_open`) and custom Java installation removal support to `JavaPage`.
- **News Jump & Fallback State**: Added quick page jump in news paginator with friendly fallback empty state and reset action when jumping to an empty page.

### Changed
- **Settings About Version Dynamic Sync**: Dynamically read package version in `SettingsAboutPage` and removed redundant feedback button in overview header.
- **Adaptive MSelect Width**: Updated `MSelect` to adapt naturally to selected text width with max-width constraint to prevent unnecessary text truncation.
- **Accordion Gap & Rounding**: Fixed spacing and corner radius on version accordion content in `CompDetailPage`.
- **DevUi Form & Segmented Alignment**: Fixed horizontal spacing between switches/checkboxes and constrained segmented control widths to card boundaries.
- **Inline MPagination in DevUi**: Added `teleport` prop control to `MPagination` to keep showcase demos rendered inline without window float-dock pollution.
- **Sidebar Logo Refinement**: Modernized sidebar header logo dimensions, drag region margin, and title alignment.
- **Streamlined Community Resource Badges**: Replaced overflowing category badge pills on `CompBrowserPage` cards with clean PCL-aligned metadata layout.

## [0.0.18] - 2026-10-25

### Added
- **Unified Floating Paginator (`MPagination`)**: Created floating dock pagination component supporting dynamic page numbers, boundary truncation with ellipsis, loading indicator, and page jumping with automatic viewport scrolling.
- **Top-to-Bottom Multi-Column News Masonry**: Replaced row grid with CSS multi-column masonry (`columns-1 md:columns-2 lg:columns-3`), eliminating vertical voids across irregular card heights.
- **Game Folders Management Dialog (`GameFoldersDialog`)**: Created dedicated folder dialog opened via toolbar cog button, supporting inline remarks, active switching, OS path reveal via Rust `path_open`, and folder removal.
- **High-Fidelity Skeleton Matching**: Rebuilt loading skeletons across all 12 core pages and panels to mirror the exact outer containers, grid columns, card sizes, and line structures of the real data.
- **Rich Markdown & HTML Detail Rendering (`MMarkdown`)**: Created unified renderer supporting embedded HTML tags (`<center>`, `<img>`, tables, rules), lazy image loading, referrer policy protection, and safe external URL opening.
- **CurseForge Keyless Access via MCIM Mirror**: Added mirror candidate resolution when no official API key is provided, enabling community search without configuration while preserving official key authentication.
- **PCL Offline Skin & Authlib Parity**: Bundled Steve & Alex default textures with automatic model resolution; added session server profile texture fetching for Authlib; added offline account renaming and UUID clipboard copying.
- **Startup Splash Animation**: Added customizable brand splash overlay on launch with smooth pop/rise keyframes and instant bypass toggle in appearance settings (`mclr.splash`).
- **Native Skin PNG Saving (`tools_save_skin`)**: Replaced browser anchor download with robust backend base64 decoding and asynchronous file writing.

### Changed
- **Shell Layout Modernization**: Extended navigation sidebar to the very top with integrated logo & app name; removed TitleBar bottom dividing line and left brand elements to eliminate redundant headers.
- **Consolidated Unified Logs Page**: Merged game process stdout/stderr viewer and launcher logs into a unified tabbed page (`/logs?tab=game|launcher`) with nav label "日志".
- **Feedback Consolidated into Settings About**: Moved issue reporting guidance and GitHub direct action into `/settings/about` immediately following the overview card.
- **Search Spinner Feedback**: Dynamic loading indicator in community search input with spinning animation.
- **Single Add/Create Instance & Account Actions**: Removed redundant in-row buttons in Launch selection cards in favor of clean header action buttons and guidance text.
- **Comprehensive UI Library Showcase (`DevUiPage`)**: Reorganized component library demo into 8 clearly defined sections with pure Chinese localization strings and zero mixed-language text.
- **Text Selectability Sweep**: Enabled `select-text` across detail headers, version numbers, paths, UUIDs, FAQ answers, toast messages, and error banners.

### Removed
- **Removed Game Folders Section from Settings System**: Cleaned up legacy settings tab entries in favor of the dedicated Instances toolbar dialog.
- **Removed Redundant Feedback & Launcher Logs from Settings System**: Removed duplicate about, feedback, and logs tabs from Settings System.

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

# MCLR Design Language — "Tonal Layers"

This document is the source of truth for MCLR's UI tokens, surface tiers, typography, motion, icons, and UI contract lint (`scripts/lint-ui-contracts.mjs`).

---

## 1. Tonal Surface & Color Tokens

CSS variables (`--c-*`) contain space-separated RGB channels (`r g b`) generated dynamically by `generatePalette(seed, mode)` in `src/ui/theme/index.ts`. `h` is the seed hue and `c = clamp(seedChroma, 0.13, 0.22)`. Neutral tiers carry a subtle seed-hue tint for chromatic cohesion.

| CSS Variable | UnoCSS Class | Light `(L, C)` | Dark `(L, C)` | Purpose |
|---|---|---|---|---|
| `--c-canvas` | `bg-canvas` | `0.965, 0.005` | `0.145, 0.012` | Window base behind all tiers |
| `--c-panel` | `bg-panel` | `0.985, 0.005` | `0.180, 0.012` | Sidebar rail & structural panels |
| `--c-raised` | `bg-raised` | `1.000, 0.000` | `0.220, 0.012` | Cards (`MCard`) |
| `--c-sunken` | `bg-sunken` | `0.938, 0.006` | `0.125, 0.012` | Inputs, wells (`.m-well`), hover fills |
| `--c-overlay` | `bg-overlay` | `1.000, 0.000` | `0.270, 0.012` | Dialogs, menus, popovers, toasts, drawers |
| `--c-line` | `border-line` | `0.885, 0.006` | `0.285, 0.012` | Hairline borders & dividers |
| `--c-line-strong` | `border-line-strong` | `0.785, 0.008` | `0.365, 0.012` | Input borders & emphasized dividers |
| `--c-fg` | `text-fg` | `0.210, 0.020` | `0.950, 0.008` | Primary text (WCAG $\ge 7:1$ vs `raised` & `canvas`) |
| `--c-fg-muted` | `text-fg-muted` | `0.460, 0.022` | `0.740, 0.012` | Secondary text (WCAG $\ge 4.5:1$ vs `raised` & `canvas`) |
| `--c-fg-subtle` | `text-fg-subtle` | `0.620, 0.020` | `0.580, 0.012` | Placeholders, disabled text, decorative icons ($\ge 3:1$ vs `raised`) |
| `--c-primary` | `bg-primary` / `text-primary` | `0.550, c` | `0.740, c` | Primary actions & active indicators |
| `--c-primary-hover` | `bg-primary-hover` | `0.500, c` | `0.790, c` | Primary hover fill |
| `--c-primary-soft` | `bg-primary-soft` | `0.940, 0.035` | `0.300, 0.060` | Tonal fills & selected nav/list rows |
| `--c-primary-soft-fg` | `text-primary-soft-fg` | `0.380, 0.130` | `0.860, 0.080` | Text/icons on `primary-soft` ($\ge 4.5:1$) |
| `--c-primary-on` | `text-primary-on` | `1.000, 0.000` | `0.170, 0.020` | Text/icons on `primary` ($\ge 4.5:1$) |
| `--c-danger` / `-soft` / `-fg` | `bg-danger`, `bg-danger-soft`, `text-danger-fg` | `h=25, C=0.20`: `0.58 / 0.95 / 0.44` | `0.70 / 0.30 / 0.80` | Error/destructive states (`-fg` vs `-soft` $\ge 4.5:1$) |
| `--c-warning` / `-soft` / `-fg` | `bg-warning`, `bg-warning-soft`, `text-warning-fg` | `h=75, C=0.15`: `0.58 / 0.95 / 0.44` | `0.70 / 0.30 / 0.80` | Warning states (`-fg` vs `-soft` $\ge 4.5:1$) |
| `--c-success` / `-soft` / `-fg` | `bg-success`, `bg-success-soft`, `text-success-fg` | `h=150, C=0.16`: `0.58 / 0.95 / 0.44` | `0.70 / 0.30 / 0.80` | Success states (`-fg` vs `-soft` $\ge 4.5:1$) |

Additional CSS variables set by `applyTheme`:
- `--shadow`: `15 23 42` (light) / `0 0 0` (dark)
- `--shadow-1`, `--shadow-2`, `--shadow-3` (UnoCSS `shadow-1`, `shadow-2`, `shadow-3`)
- `--ambient-1`, `--ambient-2`, `--ambient-3` and `--ambient-strength` (`0` in `flat`, `1` in `glass`)

---

## 2. Surface Classes & Materials (`src/ui/theme/material.css`)

- `.m-page`: Standard page container (`max-width: 1120px`, `px-8 pt-4 pb-6`, `gap-5`). All feature pages and detail sub-tabs standardize on this container to ensure 0 horizontal layout shift.
- `.m-panel`: Sidebar/navigation chrome (`panel` bg + `line` inner border in flat; translucent blurred surface in glass).
- `.m-card`: Raised card surface (`raised` bg + `line` border + `shadow-1` in flat; blurred specular card in glass). Used via `<MCard>`.
- `.m-well`: Sunken container (`sunken` bg + `line` border; opaque in both materials for legibility). Used via `<MCard variant="well">`.
- `.m-overlay`: Floating surfaces (`MDialog`, `MDrawer`, `MPopover`, `MSelect` listbox, `MContextMenu`, `MToast`, `MTooltip`).
- `.m-refract`: Liquid-glass SVG displacement refraction (`backdrop-filter: url(#mclr-refract) blur(8px) saturate(1.8)` with fallback). Used on overlay surfaces, active sidebar pill, and the hero launch button.

---

## 3. Spacing, Radius, Typography & Motion

- **Grid & Spacing**: 4 px grid; page padding `px-8 py-6` (`32px` / `24px`), section gap `24px` (`gap-6`), card padding `20px` (`p-5`), setting row min-height `52px`.
- **Radius**: `rounded-sm` (`8px`), `rounded`/`rounded-md` (`10px`), `rounded-lg` (`14px` — buttons/inputs), `rounded-xl` (`18px` — cards), `rounded-2xl` (`24px` — dialogs & drawers), `rounded-full` (badges/pills).
- **Typography**: Bundled `'HarmonyOS Sans SC'` (Regular `400`, Medium `500 600`, Bold `700`) with `'Segoe UI Variable Text', 'Microsoft YaHei UI', system-ui, sans-serif` fallbacks. Monospace uses `'Cascadia Mono', Consolas, monospace`.
- **Motion Constants**: `--dur-fast: 0.12s`, `--dur-base: 0.2s`, `--dur-slow: 0.32s`, `--ease: cubic-bezier(0.2, 0.8, 0.2, 1)`.

---

## 4. Overlay & Drawer Architecture

All floating overlays and slide-out drawers standardize on `<MOverlay>`:

- **`<MOverlay>`**: Teleported backdrop layer managing:
  - Topmost active overlay `Escape` key capture.
  - Body overflow scroll lock.
  - Backdrop blur (`backdrop-blur-xs`) and dark scrim (`bg-black/45`).
  - Alignment options: `'center'` (dialogs), `'right'` (right drawer), `'left'` (left drawer), `'bottom'`.
- **`<MDialog>`**: Modal dialog container with:
  - Variant support (`primary`, `danger`, `tonal`) on the confirm button.
  - Preset widths: `sm` (384px), `md` (480px), `lg` (560px), `xl` (680px).
  - Built-in loading state forwarding (`confirmLoading`).
- **`<MDrawer>`**: Slide-out drawer panel with:
  - Placement: `'right'` | `'left'` | `'bottom'`.
  - Preset widths: `sm` (360px), `md` (448px), `lg` (560px), `xl` (680px).
  - Transitions: `.m-drawer-right` (`translateX(100%)` → `0`), `.m-drawer-left` (`translateX(-100%)` → `0`).

---

## 5. Motion Tiers & Accessibility (`src/ui/theme/transitions.css`)

The application supports three distinct motion modes configured via `html[data-motion='full' | 'reduced' | 'off']`:

1. **Full Motion (`data-motion="full"`)**:
   - Standard spring and cubic-bezier translations, transforms, staggered entrances, and displacement refraction.
2. **Reduced Motion (`data-motion="reduced"`)**:
   - Strips translate, scale, and stagger transforms across all components, overlays, and drawers.
   - **Spinner Accessibility (`.m-pulse-reduced`)**: Rotating spinners (`.animate-spin`) are converted to a gentle opacity pulse (`animation: m-pulse-reduced 1.6s ease-in-out infinite`) so loading buttons and spinners remain visibly active without triggering vestibular discomfort.
   - Disables `MSkeleton` animated shimmer.
   - Disables `MCard` hover translate (`transform: none`).
3. **Off (`data-motion="off"`)**:
   - Forces `animation-duration: 0s !important` and `transition-duration: 0s !important` across all elements.
   - Indeterminate progress bars render static striped indicators.

### Transition Presets:
- `m-fade`: Opacity fade (160 ms).
- `m-pop`: Opacity + `scale(0.96 → 1)` (160 ms, `--origin`).
- `m-modal`: Backdrop fade + modal scale pop (200 ms).
- `m-drawer-right`: Slide from right (`translateX(100% → 0)`) 220 ms.
- `m-drawer-left`: Slide from left (`translateX(-100% → 0)`) 220 ms.
- `m-slide-up`: `translateY(12px)` + fade (220 ms).
- `m-collapse`: Height expand/collapse via CSS grid (`grid-template-rows: 0fr → 1fr`) 240 ms.

---

## 6. Standard Component Primitives

- **`<MSearchBar>`**: Standard search input wrapping `MInput` with:
  - Leading `i-lucide-search` icon.
  - Trailing clear button (`i-lucide-x`) rendered whenever the input has text.
  - Built-in loading spinner indicator.
  - Optional debounce delay.
- **`<MStatusBadge>`**: Semantic status pill wrapping `MBadge` with:
  - Color status indicator dot.
  - Semantic status types: `'online' | 'offline' | 'running' | 'waiting' | 'warning' | 'danger' | 'success'`.
- **`<InstancePreviewCard>`**: Compact instance card for launch view mirroring the layout of `AccountPreviewCard`:
  - Loader kind icon (`MLoaderIcon`) or custom block icon.
  - MC version badge, loader badges, isolation badge, and state badge.
  - Folder/path subtitle and quick action buttons (open folder, manage instance).
- **`<MListItem>`**: Enhanced list row supporting `dense`, `active`, and named slots (`#prefix`, `#title`, `#subtitle`, `#badges`, `#actions`).
- **`<MEmpty>`**: Empty state illustration with description and action slot (`#action` or `:actionText`).
- **`<MImage>`**: Universal image container replacing raw `<img>` tags across the application. Features built-in loading transitions, automatic error fallback (`fallbackSrc` or `fallbackIcon`), pixelated texture rendering support (`:pixelated="true"` for Minecraft blocks/skins), and compliant lazy decoding.

---

## 7. Icons

- UI icons use `@iconify-json/lucide` (`i-lucide-*`).
- Brand marks use `@iconify-json/simple-icons` via `<MBrandIcon>` (`i-simple-icons-{microsoft,modrinth,curseforge,github,mojangstudios,minecraft,xbox}`).
- Mod loaders use `<MLoaderIcon :kind="..." :size="24|32|40|56" />` backed by `src/ui/icons.ts`, which serves the official artwork in `public/assets/icons/loaders/` (plus `grass_block` for vanilla). Layer logos for Fabric, Forge, NeoForge, Quilt, LabyMod, OptiFine and Cleanroom come from each project's own branding; LiteLoader has no standalone official icon, so it keeps a lucide glyph tinted with its brand colour `oklch` hue 215. The tile is a **light** hue-tinted chip in both themes: loader logos are dark artwork and vanish on a dark tile. Layer artwork renders at 60–75% of the tile (larger than lucide glyphs) so brand detail survives.
- The same logos feed `MSelect` options through `image` (square, `object-contain`); `avatar` is reserved for circular skin heads.
- All icon names MUST be complete static string literals so UnoCSS can extract them from `.vue` and `.ts` files.

---

## 8. Contract Lint (`scripts/lint-ui-contracts.mjs`)

Run automatically as part of `pnpm check`:
1. Every prop and named slot used on an `<M*>` component must be declared in `src/ui/M*.vue`.
2. Legacy tokens (`text-text`, `text-text-muted`, `text-textMuted`, `text-muted`, `bg-surface`, `bg-surfaceVariant`, `bg-surface-variant`, `bg-primaryContainer`, `border-outline`, etc.) are forbidden.
3. Dynamic template-interpolated icon names (`i-lucide-${...}`) are forbidden.
4. Every color/shadow/border utility class extracted by UnoCSS must generate valid CSS.
5. Every `<img>` in `.vue` files must declare `loading` and `decoding` attributes.
6. Every `#[tauri::command]` in `src-tauri/src/ipc/*.rs` performing synchronous filesystem, archive, hashing, NBT, or image work must run inside `run_blocking(...)`.

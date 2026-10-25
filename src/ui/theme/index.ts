import { oklch, clampChroma, formatRgb, wcagContrast, type Oklch } from 'culori'

export type ThemeMode = 'light' | 'dark' | 'system'
export type MaterialMode = 'flat' | 'glass'

export interface ThemeConfig {
  seed: string
  mode: ThemeMode
  material: MaterialMode
}

export interface ResolvedRgb {
  r: number
  g: number
  b: number
}

export interface SeedPreset {
  key: 'sky' | 'indigo' | 'emerald' | 'amber' | 'rose' | 'violet' | 'teal' | 'slate'
  hex: string
}

export const SEED_PRESETS: readonly SeedPreset[] = [
  { key: 'sky', hex: '#3b82f6' },
  { key: 'indigo', hex: '#6366f1' },
  { key: 'emerald', hex: '#10b981' },
  { key: 'amber', hex: '#f59e0b' },
  { key: 'rose', hex: '#f43f5e' },
  { key: 'violet', hex: '#8b5cf6' },
  { key: 'teal', hex: '#14b8a6' },
  { key: 'slate', hex: '#64748b' },
]

export interface ThemePalette {
  canvas: string
  panel: string
  raised: string
  sunken: string
  overlay: string
  line: string
  lineStrong: string
  fg: string
  fgMuted: string
  fgSubtle: string
  primary: string
  primaryHover: string
  primarySoft: string
  primarySoftFg: string
  primaryOn: string
  danger: string
  dangerSoft: string
  dangerFg: string
  warning: string
  warningSoft: string
  warningFg: string
  success: string
  successSoft: string
  successFg: string
  shadow: string
  ambient1: string
  ambient2: string
  ambient3: string
}

function parseHexToOklch(hex: string): Oklch {
  const parsed = oklch(hex)
  if (!parsed || Number.isNaN(parsed.h) || parsed.h === undefined) {
    return { mode: 'oklch', l: 0.62, c: 0.21, h: 250 }
  }
  return parsed
}

function toneToRgb(h: number, c: number, l: number): ResolvedRgb {
  const clampedL = Math.max(0, Math.min(1, l))
  const sample = { mode: 'oklch' as const, l: clampedL, c, h: ((h % 360) + 360) % 360 }
  const clamped = clampChroma(sample, 'rgb')
  const rgbObj = formatRgb(clamped)
  if (!rgbObj) {
    return { r: 59, g: 130, b: 246 }
  }
  const match = rgbObj.match(/rgb\((\d+),\s*(\d+),\s*(\d+)\)/)
  if (match) {
    return {
      r: parseInt(match[1], 10),
      g: parseInt(match[2], 10),
      b: parseInt(match[3], 10),
    }
  }
  return { r: 59, g: 130, b: 246 }
}

function fmtRgbStr(rgb: ResolvedRgb): string {
  return `${rgb.r} ${rgb.g} ${rgb.b}`
}

function parseRgbInput(input: ResolvedRgb | string): ResolvedRgb {
  if (typeof input === 'string') {
    const parts = input.trim().split(/\s+/).map((n) => parseInt(n, 10))
    return { r: parts[0] || 0, g: parts[1] || 0, b: parts[2] || 0 }
  }
  return input
}

function toCuloriRgb(rgb: ResolvedRgb) {
  return { mode: 'rgb' as const, r: rgb.r / 255, g: rgb.g / 255, b: rgb.b / 255 }
}

function contrastRatio(a: ResolvedRgb, b: ResolvedRgb): number {
  return wcagContrast(toCuloriRgb(a), toCuloriRgb(b))
}

export function ensureContrast(fgRgb: ResolvedRgb, bgRgb: ResolvedRgb, min: number): ResolvedRgb
export function ensureContrast(fgRgb: string, bgRgb: string, min: number): string
export function ensureContrast(
  fgInput: ResolvedRgb | string,
  bgInput: ResolvedRgb | string,
  min: number,
): ResolvedRgb | string {
  const asString = typeof fgInput === 'string'
  const fgRgb = parseRgbInput(fgInput)
  const bgRgb = parseRgbInput(bgInput)

  if (contrastRatio(fgRgb, bgRgb) >= min) {
    return asString ? fmtRgbStr(fgRgb) : fgRgb
  }

  const fgOklch = oklch(toCuloriRgb(fgRgb))
  const bgOklch = oklch(toCuloriRgb(bgRgb))
  const h = fgOklch?.h ?? 0
  const c = fgOklch?.c ?? 0
  const startL = fgOklch?.l ?? 0.5
  const bgL = bgOklch?.l ?? 0.5
  const dir = startL >= bgL ? 1 : -1

  let l = startL
  let candidate = fgRgb
  for (let i = 0; i < 100; i++) {
    l = Math.max(0, Math.min(1, l + dir * 0.01))
    candidate = toneToRgb(h, c, l)
    if (contrastRatio(candidate, bgRgb) >= min) {
      return asString ? fmtRgbStr(candidate) : candidate
    }
    if (l <= 0 || l >= 1) break
  }

  l = startL
  for (let i = 0; i < 100; i++) {
    l = Math.max(0, Math.min(1, l - dir * 0.01))
    candidate = toneToRgb(h, c, l)
    if (contrastRatio(candidate, bgRgb) >= min) {
      return asString ? fmtRgbStr(candidate) : candidate
    }
    if (l <= 0 || l >= 1) break
  }

  return asString ? fmtRgbStr(candidate) : candidate
}

export function resolveEffectiveMode(mode: ThemeMode): 'light' | 'dark' {
  if (mode === 'system') {
    if (typeof window !== 'undefined' && window.matchMedia) {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return 'light'
  }
  return mode
}

export function generatePalette(seedHex: string, effectiveMode: 'light' | 'dark'): ThemePalette {
  const seedOklch = parseHexToOklch(seedHex)
  const h = seedOklch.h ?? 250
  const c = Math.max(0.13, Math.min(0.22, seedOklch.c || 0.18))
  const isDark = effectiveMode === 'dark'

  const canvasRgb = isDark ? toneToRgb(h, 0.012, 0.165) : toneToRgb(h, 0.01, 0.955)
  const panelRgb = isDark ? toneToRgb(h, 0.012, 0.195) : toneToRgb(h, 0.008, 0.975)
  const raisedRgb = isDark ? toneToRgb(h, 0.012, 0.235) : { r: 255, g: 255, b: 255 }
  const sunkenRgb = isDark ? toneToRgb(h, 0.012, 0.14) : toneToRgb(h, 0.012, 0.935)
  const overlayRgb = isDark ? toneToRgb(h, 0.012, 0.27) : { r: 255, g: 255, b: 255 }
  const lineRgb = isDark ? toneToRgb(h, 0.012, 0.3) : toneToRgb(h, 0.01, 0.905)
  const lineStrongRgb = isDark ? toneToRgb(h, 0.012, 0.38) : toneToRgb(h, 0.012, 0.84)

  let fgRgb = isDark ? toneToRgb(h, 0.008, 0.95) : toneToRgb(h, 0.02, 0.21)
  fgRgb = ensureContrast(ensureContrast(fgRgb, raisedRgb, 7), canvasRgb, 7)

  let fgMutedRgb = isDark ? toneToRgb(h, 0.012, 0.74) : toneToRgb(h, 0.022, 0.46)
  fgMutedRgb = ensureContrast(ensureContrast(fgMutedRgb, raisedRgb, 4.5), canvasRgb, 4.5)

  let fgSubtleRgb = isDark ? toneToRgb(h, 0.012, 0.58) : toneToRgb(h, 0.02, 0.62)
  fgSubtleRgb = ensureContrast(fgSubtleRgb, raisedRgb, 3)

  let primaryRgb = isDark ? toneToRgb(h, c, 0.74) : toneToRgb(h, c, 0.55)
  const primaryHoverRgb = isDark ? toneToRgb(h, c, 0.79) : toneToRgb(h, c, 0.5)
  const primarySoftRgb = isDark ? toneToRgb(h, 0.06, 0.3) : toneToRgb(h, 0.035, 0.94)

  let primarySoftFgRgb = isDark ? toneToRgb(h, 0.08, 0.86) : toneToRgb(h, 0.13, 0.38)
  primarySoftFgRgb = ensureContrast(primarySoftFgRgb, primarySoftRgb, 4.5)

  let primaryOnRgb = isDark ? toneToRgb(h, 0.02, 0.17) : { r: 255, g: 255, b: 255 }
  if (!isDark) {
    primaryRgb = ensureContrast(primaryRgb, primaryOnRgb, 4.5)
  }
  primaryOnRgb = ensureContrast(primaryOnRgb, primaryRgb, 4.5)

  // Status triples: danger (h25 C.20), warning (h75 C.15), success (h150 C.16)
  const dangerRgb = isDark ? toneToRgb(25, 0.2, 0.7) : toneToRgb(25, 0.2, 0.58)
  const dangerSoftRgb = isDark ? toneToRgb(25, 0.03, 0.3) : toneToRgb(25, 0.03, 0.95)
  const dangerFgRgb = ensureContrast(
    isDark ? toneToRgb(25, 0.2, 0.8) : toneToRgb(25, 0.2, 0.44),
    dangerSoftRgb,
    4.5,
  )

  const warningRgb = isDark ? toneToRgb(75, 0.15, 0.7) : toneToRgb(75, 0.15, 0.58)
  const warningSoftRgb = isDark ? toneToRgb(75, 0.03, 0.3) : toneToRgb(75, 0.03, 0.95)
  const warningFgRgb = ensureContrast(
    isDark ? toneToRgb(75, 0.15, 0.8) : toneToRgb(75, 0.15, 0.44),
    warningSoftRgb,
    4.5,
  )

  const successRgb = isDark ? toneToRgb(150, 0.16, 0.7) : toneToRgb(150, 0.16, 0.58)
  const successSoftRgb = isDark ? toneToRgb(150, 0.03, 0.3) : toneToRgb(150, 0.03, 0.95)
  const successFgRgb = ensureContrast(
    isDark ? toneToRgb(150, 0.16, 0.8) : toneToRgb(150, 0.16, 0.44),
    successSoftRgb,
    4.5,
  )

  const shadow = isDark ? '0 0 0' : '24 32 56'

  const ambient1Rgb = isDark ? toneToRgb(h, 0.1, 0.36) : toneToRgb(h, 0.11, 0.8)
  const ambient2Rgb = isDark ? toneToRgb(h + 45, 0.1, 0.34) : toneToRgb(h + 45, 0.11, 0.82)
  const ambient3Rgb = isDark ? toneToRgb(h - 35, 0.1, 0.32) : toneToRgb(h - 35, 0.11, 0.84)

  return {
    canvas: fmtRgbStr(canvasRgb),
    panel: fmtRgbStr(panelRgb),
    raised: fmtRgbStr(raisedRgb),
    sunken: fmtRgbStr(sunkenRgb),
    overlay: fmtRgbStr(overlayRgb),
    line: fmtRgbStr(lineRgb),
    lineStrong: fmtRgbStr(lineStrongRgb),
    fg: fmtRgbStr(fgRgb),
    fgMuted: fmtRgbStr(fgMutedRgb),
    fgSubtle: fmtRgbStr(fgSubtleRgb),
    primary: fmtRgbStr(primaryRgb),
    primaryHover: fmtRgbStr(primaryHoverRgb),
    primarySoft: fmtRgbStr(primarySoftRgb),
    primarySoftFg: fmtRgbStr(primarySoftFgRgb),
    primaryOn: fmtRgbStr(primaryOnRgb),
    danger: fmtRgbStr(dangerRgb),
    dangerSoft: fmtRgbStr(dangerSoftRgb),
    dangerFg: fmtRgbStr(dangerFgRgb),
    warning: fmtRgbStr(warningRgb),
    warningSoft: fmtRgbStr(warningSoftRgb),
    warningFg: fmtRgbStr(warningFgRgb),
    success: fmtRgbStr(successRgb),
    successSoft: fmtRgbStr(successSoftRgb),
    successFg: fmtRgbStr(successFgRgb),
    shadow,
    ambient1: fmtRgbStr(ambient1Rgb),
    ambient2: fmtRgbStr(ambient2Rgb),
    ambient3: fmtRgbStr(ambient3Rgb),
  }
}

export function applyTheme(config: ThemeConfig): void {
  if (typeof document === 'undefined') return

  const effectiveMode = resolveEffectiveMode(config.mode)
  const isDark = effectiveMode === 'dark'
  const root = document.documentElement

  root.dataset.mode = effectiveMode
  root.dataset.material = config.material

  const palette = generatePalette(config.seed, effectiveMode)

  for (const [key, value] of Object.entries(palette)) {
    const kebab = key
      .replace(/([a-z0-9])([A-Z])/g, '$1-$2')
      .replace(/([a-zA-Z])([0-9])/g, '$1-$2')
      .toLowerCase()
    root.style.setProperty(`--c-${kebab}`, value)
  }

  root.style.setProperty('--shadow', palette.shadow)
  root.style.setProperty('--ambient-1', palette.ambient1)
  root.style.setProperty('--ambient-2', palette.ambient2)
  root.style.setProperty('--ambient-3', palette.ambient3)
  root.style.setProperty('--ambient-strength', config.material === 'glass' ? '1' : '0.55')

  if (isDark) {
    root.style.setProperty(
      '--shadow-1',
      '0 1px 3px 0 rgb(var(--shadow) / 0.2), 0 1px 2px -1px rgb(var(--shadow) / 0.15)',
    )
    root.style.setProperty(
      '--shadow-2',
      '0 4px 14px -2px rgb(var(--shadow) / 0.28), 0 2px 6px -1px rgb(var(--shadow) / 0.18)',
    )
    root.style.setProperty(
      '--shadow-3',
      '0 16px 32px -4px rgb(var(--shadow) / 0.4), 0 4px 12px -2px rgb(var(--shadow) / 0.2)',
    )
  } else {
    root.style.setProperty(
      '--shadow-1',
      '0 1px 3px 0 rgb(var(--shadow) / 0.06), 0 1px 2px -1px rgb(var(--shadow) / 0.04)',
    )
    root.style.setProperty(
      '--shadow-2',
      '0 4px 14px -2px rgb(var(--shadow) / 0.08), 0 2px 6px -1px rgb(var(--shadow) / 0.04)',
    )
    root.style.setProperty(
      '--shadow-3',
      '0 16px 32px -4px rgb(var(--shadow) / 0.12), 0 4px 12px -2px rgb(var(--shadow) / 0.06)',
    )
  }
}

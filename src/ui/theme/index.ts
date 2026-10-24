import { oklch, clampChroma, formatRgb, type Oklch } from 'culori'

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

function parseHexToOklch(hex: string): Oklch {
  const parsed = oklch(hex)
  if (!parsed || Number.isNaN(parsed.h) || parsed.h === undefined) {
    // Default blue #3b82f6 oklch fallback
    return { mode: 'oklch', l: 0.62, c: 0.21, h: 250 }
  }
  return parsed
}

function toneToRgb(h: number, c: number, l: number): ResolvedRgb {
  // Clamp lightness 0..1
  const clampedL = Math.max(0, Math.min(1, l))
  const sample = { mode: 'oklch' as const, l: clampedL, c, h }
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

export function resolveEffectiveMode(mode: ThemeMode): 'light' | 'dark' {
  if (mode === 'system') {
    if (typeof window !== 'undefined' && window.matchMedia) {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return 'light'
  }
  return mode
}

export function generatePalette(seedHex: string, effectiveMode: 'light' | 'dark') {
  const seedOklch = parseHexToOklch(seedHex)
  const h = seedOklch.h ?? 250
  const c = Math.max(0.12, Math.min(0.24, seedOklch.c || 0.18))

  const isDark = effectiveMode === 'dark'

  const primaryRgb = isDark
    ? toneToRgb(h, c, 0.72)
    : toneToRgb(h, c, 0.52)

  const onPrimaryRgb = isDark
    ? toneToRgb(h, 0.02, 0.12)
    : { r: 255, g: 255, b: 255 }

  const primaryContainerRgb = isDark
    ? toneToRgb(h, c * 0.7, 0.32)
    : toneToRgb(h, c * 0.45, 0.90)

  const surfaceRgb = isDark
    ? toneToRgb(h, 0.015, 0.13)
    : toneToRgb(h, 0.01, 0.99)

  const surfaceVariantRgb = isDark
    ? toneToRgb(h, 0.025, 0.18)
    : toneToRgb(h, 0.02, 0.94)

  const outlineRgb = isDark
    ? toneToRgb(h, 0.02, 0.30)
    : toneToRgb(h, 0.02, 0.82)

  const textRgb = isDark
    ? toneToRgb(h, 0.01, 0.95)
    : toneToRgb(h, 0.01, 0.10)

  const textMutedRgb = isDark
    ? toneToRgb(h, 0.02, 0.65)
    : toneToRgb(h, 0.02, 0.45)

  // Status colors (fixed hues: danger ~25, warning ~75, success ~145)
  const dangerRgb = isDark ? toneToRgb(25, 0.22, 0.68) : toneToRgb(25, 0.22, 0.55)
  const warningRgb = isDark ? toneToRgb(75, 0.18, 0.72) : toneToRgb(75, 0.18, 0.58)
  const successRgb = isDark ? toneToRgb(145, 0.18, 0.70) : toneToRgb(145, 0.18, 0.55)

  return {
    primary: fmtRgbStr(primaryRgb),
    onPrimary: fmtRgbStr(onPrimaryRgb),
    primaryContainer: fmtRgbStr(primaryContainerRgb),
    surface: fmtRgbStr(surfaceRgb),
    surfaceVariant: fmtRgbStr(surfaceVariantRgb),
    outline: fmtRgbStr(outlineRgb),
    text: fmtRgbStr(textRgb),
    textMuted: fmtRgbStr(textMutedRgb),
    danger: fmtRgbStr(dangerRgb),
    warning: fmtRgbStr(warningRgb),
    success: fmtRgbStr(successRgb),
  }
}

export function applyTheme(config: ThemeConfig): void {
  if (typeof document === 'undefined') return

  const effectiveMode = resolveEffectiveMode(config.mode)
  const root = document.documentElement

  root.dataset.mode = effectiveMode
  root.dataset.material = config.material

  const palette = generatePalette(config.seed, effectiveMode)

  root.style.setProperty('--c-primary', palette.primary)
  root.style.setProperty('--c-on-primary', palette.onPrimary)
  root.style.setProperty('--c-primary-container', palette.primaryContainer)
  root.style.setProperty('--c-surface', palette.surface)
  root.style.setProperty('--c-surface-variant', palette.surfaceVariant)
  root.style.setProperty('--c-outline', palette.outline)
  root.style.setProperty('--c-text', palette.text)
  root.style.setProperty('--c-text-muted', palette.textMuted)
  root.style.setProperty('--c-danger', palette.danger)
  root.style.setProperty('--c-warning', palette.warning)
  root.style.setProperty('--c-success', palette.success)
}

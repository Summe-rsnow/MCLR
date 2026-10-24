import { describe, expect, it } from 'vitest'
import { wcagContrast } from 'culori'
import { generatePalette, SEED_PRESETS, applyTheme } from '../index'

function rgbStrToCulori(rgbStr: string) {
  const [r, g, b] = rgbStr.trim().split(/\s+/).map((n) => parseInt(n, 10) / 255)
  return { mode: 'rgb' as const, r, g, b }
}

describe('Tonal Layers Palette & WCAG Contrast', () => {
  for (const preset of SEED_PRESETS) {
    for (const mode of ['light', 'dark'] as const) {
      it(`meets all contrast targets for preset "${preset.key}" (${preset.hex}) in ${mode} mode`, () => {
        const p = generatePalette(preset.hex, mode)

        const canvas = rgbStrToCulori(p.canvas)
        const raised = rgbStrToCulori(p.raised)
        const fg = rgbStrToCulori(p.fg)
        const fgMuted = rgbStrToCulori(p.fgMuted)
        const fgSubtle = rgbStrToCulori(p.fgSubtle)
        const primary = rgbStrToCulori(p.primary)
        const primaryOn = rgbStrToCulori(p.primaryOn)
        const primarySoft = rgbStrToCulori(p.primarySoft)
        const primarySoftFg = rgbStrToCulori(p.primarySoftFg)

        expect(wcagContrast(fg, raised)).toBeGreaterThanOrEqual(7)
        expect(wcagContrast(fg, canvas)).toBeGreaterThanOrEqual(7)
        expect(wcagContrast(fgMuted, raised)).toBeGreaterThanOrEqual(4.5)
        expect(wcagContrast(fgMuted, canvas)).toBeGreaterThanOrEqual(4.5)
        expect(wcagContrast(fgSubtle, raised)).toBeGreaterThanOrEqual(3)
        expect(wcagContrast(primaryOn, primary)).toBeGreaterThanOrEqual(4.5)
        expect(wcagContrast(primarySoftFg, primarySoft)).toBeGreaterThanOrEqual(4.5)

        expect(
          wcagContrast(rgbStrToCulori(p.dangerFg), rgbStrToCulori(p.dangerSoft)),
        ).toBeGreaterThanOrEqual(4.5)
        expect(
          wcagContrast(rgbStrToCulori(p.warningFg), rgbStrToCulori(p.warningSoft)),
        ).toBeGreaterThanOrEqual(4.5)
        expect(
          wcagContrast(rgbStrToCulori(p.successFg), rgbStrToCulori(p.successSoft)),
        ).toBeGreaterThanOrEqual(4.5)
      })
    }
  }

  it('writes all CSS variables and mode/material dataset attributes via applyTheme', () => {
    applyTheme({ seed: '#3b82f6', mode: 'light', material: 'flat' })
    const root = document.documentElement
    expect(root.dataset.mode).toBe('light')
    expect(root.dataset.material).toBe('flat')
    expect(root.style.getPropertyValue('--c-canvas')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-panel')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-raised')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-sunken')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-overlay')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-line')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-line-strong')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-fg')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-fg-muted')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-fg-subtle')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-primary-soft')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-primary-soft-fg')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--c-primary-on')).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(root.style.getPropertyValue('--ambient-strength')).toBe('0.55')
    expect(root.style.getPropertyValue('--shadow-1')).toContain('rgb(var(--shadow)')

    applyTheme({ seed: '#6366f1', mode: 'dark', material: 'glass' })
    expect(root.dataset.mode).toBe('dark')
    expect(root.dataset.material).toBe('glass')
    expect(root.style.getPropertyValue('--ambient-strength')).toBe('1')
  })
})

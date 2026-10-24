import { describe, expect, it } from 'vitest'
import { createGenerator } from 'unocss'
import fs from 'node:fs'
import path from 'node:path'
import config from '../../../uno.config'

function collectSourceFiles(dir: string): string[] {
  const results: string[] = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) {
      results.push(...collectSourceFiles(full))
    } else if (/\.(vue|ts)$/.test(entry.name) && !entry.name.endsWith('.d.ts')) {
      results.push(full)
    }
  }
  return results
}

describe('UnoCSS Configuration & Icon Generation', () => {
  it('content.pipeline.include matches .ts route files', () => {
    const pipeline = config.content?.pipeline
    expect(pipeline && typeof pipeline === 'object').toBe(true)
    const includePatterns = pipeline && typeof pipeline === 'object' ? pipeline.include : undefined
    expect(includePatterns).toBeDefined()
    const patterns = Array.isArray(includePatterns) ? includePatterns : [includePatterns]
    const matched = patterns.some((p) =>
      p instanceof RegExp ? p.test('src/features/home/routes.ts') : false,
    )
    expect(matched).toBe(true)
  })

  it('generates CSS for every static i-lucide-* and i-simple-icons-* in src/**/*.{vue,ts}', async () => {
    const srcDir = path.resolve(__dirname, '../..')
    const files = collectSourceFiles(srcDir)
    const iconSet = new Set<string>()
    const iconRegex = /\bi-(?:lucide|simple-icons)-[a-z0-9-]+\b/g

    for (const file of files) {
      const content = fs.readFileSync(file, 'utf8')
      for (const match of content.matchAll(iconRegex)) {
        // Ignore trailing dash from template literal prefix if any (caught by contract lint)
        if (!match[0].endsWith('-')) {
          iconSet.add(match[0])
        }
      }
    }

    expect(iconSet.size).toBeGreaterThan(0)
    const uno = await createGenerator(config)
    const allIcons = Array.from(iconSet)
    const { matched } = await uno.generate(allIcons.join(' '), { preflights: false })

    const missing = allIcons.filter((icon) => !matched.has(icon))
    expect(missing, `Unknown icon names failed to generate CSS: ${missing.join(', ')}`).toEqual([])
  })

  it('applies custom radius, font, shadow and tonal color theme tokens', async () => {
    const uno = await createGenerator(config)
    const { css, matched } = await uno.generate(
      'rounded-xl font-sans shadow-md shadow-1 bg-canvas bg-panel bg-raised bg-sunken text-fg text-fg-muted border-line',
    )
    expect(css).toContain('18px')
    expect(css).toContain('HarmonyOS Sans SC')
    expect(matched.has('shadow-md')).toBe(true)
    expect(matched.has('shadow-1')).toBe(true)
    expect(matched.has('bg-canvas')).toBe(true)
    expect(matched.has('text-fg-muted')).toBe(true)
    expect(css).not.toContain('<alpha-value>')
  })
})

import { defineConfig, presetIcons, presetWind4 } from 'unocss'

export default defineConfig({
  content: {
    pipeline: {
      include: [/\.(vue|ts)($|\?)/],
      exclude: [/node_modules/, /\.d\.ts$/],
    },
  },
  presets: [
    presetWind4(),
    presetIcons({
      scale: 1.2,
      warn: true,
    }),
  ],
  theme: {
    colors: {
      canvas: 'rgb(var(--c-canvas, 242 244 247))',
      panel: 'rgb(var(--c-panel, 248 249 251))',
      raised: 'rgb(var(--c-raised, 255 255 255))',
      sunken: 'rgb(var(--c-sunken, 236 239 243))',
      overlay: 'rgb(var(--c-overlay, 255 255 255))',
      line: {
        DEFAULT: 'rgb(var(--c-line, 226 230 236))',
        strong: 'rgb(var(--c-line-strong, 207 212 220))',
      },
      fg: {
        DEFAULT: 'rgb(var(--c-fg, 19 25 36))',
        muted: 'rgb(var(--c-fg-muted, 84 95 112))',
        subtle: 'rgb(var(--c-fg-subtle, 130 140 155))',
      },
      primary: {
        DEFAULT: 'rgb(var(--c-primary, 59 130 246))',
        hover: 'rgb(var(--c-primary-hover, 37 99 235))',
        soft: 'rgb(var(--c-primary-soft, 231 241 254))',
        'soft-fg': 'rgb(var(--c-primary-soft-fg, 25 74 166))',
        on: 'rgb(var(--c-primary-on, 255 255 255))',
      },
      danger: {
        DEFAULT: 'rgb(var(--c-danger, 229 72 77))',
        soft: 'rgb(var(--c-danger-soft, 254 235 235))',
        fg: 'rgb(var(--c-danger-fg, 168 36 41))',
      },
      warning: {
        DEFAULT: 'rgb(var(--c-warning, 217 119 6))',
        soft: 'rgb(var(--c-warning-soft, 254 243 226))',
        fg: 'rgb(var(--c-warning-fg, 138 78 12))',
      },
      success: {
        DEFAULT: 'rgb(var(--c-success, 22 163 74))',
        soft: 'rgb(var(--c-success-soft, 230 249 236))',
        fg: 'rgb(var(--c-success-fg, 20 110 52))',
      },
    },
    radius: {
      sm: '8px',
      DEFAULT: '10px',
      md: '10px',
      lg: '14px',
      xl: '18px',
      '2xl': '24px',
    },
    font: {
      sans: "'HarmonyOS Sans SC','Segoe UI Variable Text','Microsoft YaHei UI',system-ui,sans-serif",
      mono: "'Cascadia Mono',Consolas,monospace",
    },
    shadow: {
      1: 'var(--shadow-1)',
      2: 'var(--shadow-2)',
      3: 'var(--shadow-3)',
      sm: 'var(--shadow-1)',
      DEFAULT: 'var(--shadow-1)',
      md: 'var(--shadow-2)',
      lg: 'var(--shadow-3)',
    },
  },
})

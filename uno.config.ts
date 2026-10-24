import { defineConfig, presetIcons, presetWind4 } from 'unocss'

export default defineConfig({
  presets: [
    presetWind4(),
    presetIcons({
      scale: 1.2,
      warn: true,
    }),
  ],
  theme: {
    colors: {
      primary: 'rgb(var(--c-primary, 59 130 246) / <alpha-value>)',
      onPrimary: 'rgb(var(--c-on-primary, 255 255 255) / <alpha-value>)',
      primaryContainer: 'rgb(var(--c-primary-container, 219 234 254) / <alpha-value>)',
      surface: 'rgb(var(--c-surface, 255 255 255) / <alpha-value>)',
      surfaceVariant: 'rgb(var(--c-surface-variant, 241 245 249) / <alpha-value>)',
      outline: 'rgb(var(--c-outline, 203 213 225) / <alpha-value>)',
      text: 'rgb(var(--c-text, 15 23 42) / <alpha-value>)',
      textMuted: 'rgb(var(--c-text-muted, 100 116 139) / <alpha-value>)',
      danger: 'rgb(var(--c-danger, 239 68 68) / <alpha-value>)',
      warning: 'rgb(var(--c-warning, 245 158 11) / <alpha-value>)',
      success: 'rgb(var(--c-success, 34 197 94) / <alpha-value>)',
    },
  },
})

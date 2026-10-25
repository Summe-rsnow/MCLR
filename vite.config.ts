import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import UnoCSS from 'unocss/vite'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [vue(), UnoCSS()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**', '**/target/**', '**/crates/**'],
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return undefined
          if (/\/(vue|@vue|vue-router|pinia|vue-i18n|@intlify)\//.test(id)) return 'vendor-vue'
          if (/\/(markdown-it|dompurify|entities|linkify-it|mdurl|uc\.micro)\//.test(id))
            return 'vendor-md'
          if (id.includes('/culori/')) return 'vendor-color'
          if (id.includes('@tanstack/vue-virtual') || id.includes('@tanstack/virtual-core'))
            return 'vendor-virtual'
          return undefined
        },
      },
    },
  },
  test: {
    environment: 'happy-dom',
    globals: true,
  },
})

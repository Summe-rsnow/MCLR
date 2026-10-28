import './ui/theme/fonts.css'
import 'uno.css'
import './ui/theme/material.css'
import './ui/theme/transitions.css'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { i18n } from './i18n'
import { router } from './router'
import { useThemeStore } from './stores/theme'

// Demo data is dev-only: production bundles never contain mock IPC. A static import
// cannot work here — the dynamic import is what lets Vite drop the mock graph from prod.
if (import.meta.env.DEV && import.meta.env.VITE_MOCK_IPC?.trim() === '1') {
  const { setupMockIpc } = await import('./ipc/mock')
  setupMockIpc()
}

/**
 * Suppresses browser behaviours that make a packaged desktop app feel like a
 * web page: the native WebView2 context menu (Reload / Save as / Print), page
 * reload keys, and the inspector shortcuts.
 *
 * Dev builds keep everything so the inspector and "Reload" stay available.
 */
function installDesktopGuards() {
  // Editable fields keep their native menu (paste / spellcheck suggestions).
  const isEditable = (target: EventTarget | null) => {
    const el = target as HTMLElement | null
    if (!el || typeof el.closest !== 'function') return false
    return Boolean(el.closest('input, textarea, [contenteditable="true"], [contenteditable=""]'))
  }

  window.addEventListener(
    'contextmenu',
    (e) => {
      if (isEditable(e.target)) return
      e.preventDefault()
    },
    { capture: true },
  )

  window.addEventListener(
    'keydown',
    (e) => {
      const key = e.key.toLowerCase()
      const ctrl = e.ctrlKey || e.metaKey
      const reload = key === 'f5' || (ctrl && key === 'r')
      const inspector =
        key === 'f12' || (ctrl && e.shiftKey && (key === 'i' || key === 'j' || key === 'c'))
      if (reload || inspector) {
        e.preventDefault()
        e.stopPropagation()
      }
    },
    { capture: true },
  )
}

if (import.meta.env.PROD) {
  installDesktopGuards()
}

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(i18n)
app.use(router)

const themeStore = useThemeStore(pinia)
themeStore.init()

app.mount('#app')

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
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
if (import.meta.env.DEV && !isTauri && import.meta.env.VITE_MOCK_IPC?.trim() === '1') {
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

/**
 * Text is selectable by default (`user-select: text`), including inside cards
 * that open on click. Without this guard, drag-selecting a card's description
 * would also fire its click and navigate away mid-selection.
 *
 * Real controls are exempt, so clicking a button right after selecting text
 * elsewhere still works.
 */
const CONTROL_SELECTOR =
  'button, a, input, textarea, select, label, summary, [role="button"], [role="tab"], [role="menuitem"], [role="option"], [contenteditable="true"]'

function installSelectionClickGuard() {
  document.addEventListener(
    'click',
    (e) => {
      const selection = window.getSelection()
      if (!selection || selection.isCollapsed || !selection.toString()) return
      const target = e.target
      if (!(target instanceof Element) || target.closest(CONTROL_SELECTOR)) return
      e.preventDefault()
      e.stopPropagation()
    },
    { capture: true },
  )
}

installSelectionClickGuard()

if (import.meta.env.PROD) {
  installDesktopGuards()
}

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(i18n)
app.use(router)

import { ensureFormatCulture } from './composables/useFormat'
import { ensureLocale } from './composables/useLocale'

const themeStore = useThemeStore(pinia)
themeStore.init()

// Resolve the persisted UI language before the first paint so the choice survives
// a restart instead of flashing back to the default locale.
await Promise.all([ensureLocale(), ensureFormatCulture()])

app.mount('#app')

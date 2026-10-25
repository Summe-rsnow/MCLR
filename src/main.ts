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

const app = createApp(App)
const pinia = createPinia()
app.use(pinia)
app.use(i18n)
app.use(router)

const themeStore = useThemeStore(pinia)
themeStore.init()

app.mount('#app')

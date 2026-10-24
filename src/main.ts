import './ui/theme/fonts.css'
import 'uno.css'
import './ui/theme/material.css'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { i18n } from './i18n'
import { router } from './router'
import { setupMockIpc } from './ipc/mock'
import { useThemeStore } from './stores/theme'

if (
  import.meta.env.VITE_MOCK_IPC?.trim() === '1' ||
  (typeof window !== 'undefined' && !('__TAURI_INTERNALS__' in window))
) {
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

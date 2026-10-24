import 'uno.css'
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { i18n } from './i18n'
import { router } from './router'
import { setupMockIpc } from './ipc/mock'

if (import.meta.env.VITE_MOCK_IPC === '1') {
  setupMockIpc()
}

const app = createApp(App)
app.use(createPinia())
app.use(i18n)
app.use(router)
app.mount('#app')

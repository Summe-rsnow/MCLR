import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import App from './App.vue'
import { i18n } from './i18n'
import { router } from './router'
import { setupMockIpc } from './ipc/mock'

describe('App foundation', () => {
  it('renders application title via i18n', async () => {
    setupMockIpc()
    const wrapper = mount(App, {
      global: {
        plugins: [createPinia(), i18n, router],
      },
    })
    expect(wrapper.text()).toContain('MCLR')
  })
})

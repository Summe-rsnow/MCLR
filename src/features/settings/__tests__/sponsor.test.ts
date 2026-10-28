import { describe, it, expect, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import { existsSync } from 'node:fs'
import { resolve } from 'node:path'
import SettingsSponsorPage from '../SettingsSponsorPage.vue'
import zhCnSettings from '../../../locales/zh-CN/settings.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: { 'zh-CN': { settings: zhCnSettings } },
})

describe('SettingsSponsorPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders both payment channels with their QR codes', () => {
    const wrapper = mount(SettingsSponsorPage, { global: { plugins: [i18n] } })

    expect(wrapper.find('[data-testid="settings-sponsor-page"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('赞助 MCLR')
    expect(wrapper.text()).toContain('微信赞赏')
    expect(wrapper.text()).toContain('支付宝赞赏')

    const sources = wrapper.findAll('img').map((img) => img.attributes('src'))
    expect(sources).toContain('/sponsor/wechat.png')
    expect(sources).toContain('/sponsor/alipay.png')
  })

  it('ships the QR images as static assets', () => {
    // A missing file would render an empty card in production, not a build error.
    expect(existsSync(resolve(__dirname, '../../../../public/sponsor/wechat.png'))).toBe(true)
    expect(existsSync(resolve(__dirname, '../../../../public/sponsor/alipay.png'))).toBe(true)
  })

  it('keeps the sponsor copy voluntary and promise-free', () => {
    const wrapper = mount(SettingsSponsorPage, { global: { plugins: [i18n] } })
    expect(wrapper.text()).toContain('赞助完全自愿')
    expect(wrapper.text()).not.toContain('不会解锁任何功能')
    // The section no longer claims a purpose (signing / distribution costs).
    expect(wrapper.text()).not.toContain('构建分发成本')
    // No save action: the QR is meant to be scanned, nothing else.
    expect(wrapper.find('[data-testid="sponsor-save-wechat"]').exists()).toBe(false)
    expect(wrapper.text()).not.toContain('保存二维码')
  })
})

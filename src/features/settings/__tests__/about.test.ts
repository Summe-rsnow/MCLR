import { describe, it, expect, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { createI18n } from 'vue-i18n'
import SettingsAboutPage from '../SettingsAboutPage.vue'
import zhCnSettings from '../../../locales/zh-CN/settings.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import pkg from '../../../../package.json'
const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      settings: zhCnSettings,
      common: zhCnCommon,
    },
  },
})

describe('SettingsAboutPage & Help Center', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('renders app version badge, license, and description', () => {
    const wrapper = mount(SettingsAboutPage, {
      global: {
        plugins: [i18n],
      },
    })

    expect(wrapper.text()).toContain(`v${pkg.version}`)
    expect(wrapper.text()).toContain('Apache-2.0')
    expect(wrapper.text()).toContain('MCLR')
    expect(wrapper.text()).toContain('GitHub 仓库')
  })

  it('renders FAQ & Help Guide section with four common question accordions', async () => {
    const wrapper = mount(SettingsAboutPage, {
      global: {
        plugins: [i18n],
      },
    })

    // Assert FAQ section title
    expect(wrapper.text()).toContain('常见问题与使用帮助')

    // Assert 4 questions exist
    expect(wrapper.text()).toContain('账号添加与登录模式')
    expect(wrapper.text()).toContain('模组安装与加载器选择')
    expect(wrapper.text()).toContain('Java 版本匹配与自动下载')
    expect(wrapper.text()).toContain('游戏闪退与崩溃排查')

    // By default, account FAQ is expanded
    const accountContent = wrapper.find('[data-testid="faq-content-account"]')
    expect(accountContent.exists()).toBe(true)
    expect(accountContent.text()).toContain('微软正版账号')

    // Click on mods accordion to toggle open
    const modsBtn = wrapper.find('[data-testid="faq-btn-mods"]')
    await modsBtn.trigger('click')
    await flushPromises()

    const modsContent = wrapper.find('[data-testid="faq-content-mods"]')
    expect(modsContent.exists()).toBe(true)
    expect(modsContent.text()).toContain('模组加载器安装')
    expect(modsContent.text()).toContain('Fabric')

    // Click on java accordion to toggle open
    const javaBtn = wrapper.find('[data-testid="faq-btn-java"]')
    await javaBtn.trigger('click')
    await flushPromises()

    const javaContent = wrapper.find('[data-testid="faq-content-java"]')
    expect(javaContent.exists()).toBe(true)
    expect(javaContent.text()).toContain('Java 8')
    expect(javaContent.text()).toContain('Java 21')

    // Click on crash accordion to toggle open
    const crashBtn = wrapper.find('[data-testid="faq-btn-crash"]')
    await crashBtn.trigger('click')
    await flushPromises()

    const crashContent = wrapper.find('[data-testid="faq-content-crash"]')
    expect(crashContent.exists()).toBe(true)
    expect(crashContent.text()).toContain('查看日志与崩溃分析')
  })

  it('renders feedback section with guidance and github issue button', () => {
    const wrapper = mount(SettingsAboutPage, {
      global: {
        plugins: [i18n],
      },
    })

    expect(wrapper.text()).toContain('问题反馈')
    expect(wrapper.text()).toContain('提交问题前建议检查')
    expect(wrapper.text()).toContain('前往 GitHub 提交反馈')
    const btn = wrapper.find('[data-testid="open-github-btn"]')
    expect(btn.exists()).toBe(true)
  })
})

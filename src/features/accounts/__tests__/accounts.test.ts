import { describe, it, expect, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import { useAccountsStore } from '../store'
import AccountsPage from '../AccountsPage.vue'
import { setupMockIpc } from '../../../ipc/mock'
import { resetMockAccounts } from '../mock'
import zhCnAccounts from '../../../locales/zh-CN/accounts.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'
import zhCnError from '../../../locales/zh-CN/error.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      accounts: zhCnAccounts,
      common: zhCnCommon,
      error: zhCnError,
    },
  },
})

describe('Accounts feature', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    resetMockAccounts()
    setupMockIpc()
  })

  it('loads accounts and has a selected account', async () => {
    const store = useAccountsStore()
    await store.fetchAccounts()

    expect(store.accounts.length).toBeGreaterThan(0)
    expect(store.selectedAccount).not.toBeNull()
    expect(store.selectedAccount?.username).toBe('StevePro')
  })

  it('adds an offline account and switches accounts', async () => {
    const store = useAccountsStore()
    await store.fetchAccounts()
    const initialCount = store.accounts.length

    const created = await store.addOffline('TestPlayer', 'standard')
    expect(created.username).toBe('TestPlayer')
    expect(created.kind).toBe('offline')
    expect(store.accounts.length).toBe(initialCount + 1)

    // Switch selection
    await store.selectAccount(created.id)
    expect(store.selectedAccount?.id).toBe(created.id)
  })

  it('removes an account', async () => {
    const store = useAccountsStore()
    await store.fetchAccounts()
    const target = store.accounts[1]

    await store.removeAccount(target.id)
    expect(store.accounts.some((a) => a.id === target.id)).toBe(false)
  })

  it('supports skin upload and cape toggle', async () => {
    const store = useAccountsStore()
    await store.fetchAccounts()
    const target = store.accounts[0]

    // Upload skin
    const updatedSkin = await store.uploadSkin(target.id, 'slim', 'C:/path/skin.png')
    expect(updatedSkin.defaultSkinModel).toBe('slim')
    expect(updatedSkin.skinHeadDataUrl).toBeTruthy()

    // Cape toggle
    const updatedCape = await store.setCape(target.id, 'cape-vanilla')
    expect(updatedCape.capes.find((c) => c.id === 'cape-vanilla')?.active).toBe(true)
    expect(updatedCape.capes.find((c) => c.id === 'cape-migrator')?.active).toBe(false)
  })

  it('renders AccountsPage and displays accounts list', async () => {
    const wrapper = mount(AccountsPage, {
      global: {
        plugins: [i18n],
      },
    })

    await flushPromises()

    expect(wrapper.text()).toContain('StevePro')
    expect(wrapper.text()).toContain('AlexBuilder')
    expect(wrapper.text()).toContain('SkinUser01')
    expect(wrapper.text()).toContain('皮肤与披风管理')
  })
})

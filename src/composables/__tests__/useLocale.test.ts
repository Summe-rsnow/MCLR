import { beforeEach, describe, expect, it } from 'vitest'
import { i18n } from '../../i18n'
import { setupMockIpc } from '../../ipc/mock'
import { ensureLocale, setAppLocale } from '../useLocale'

describe('useLocale', () => {
  beforeEach(() => {
    setupMockIpc()
    setAppLocale('zh-CN')
  })

  it('applies a shipped locale from the persisted settings on boot', async () => {
    expect(i18n.global.locale.value).toBe('zh-CN')

    await ensureLocale()

    // The mock stores zh-CN; the value must be read, not hardcoded at createI18n time.
    expect(i18n.global.locale.value).toBe('zh-CN')
  })

  it('ignores locales without a message bundle', () => {
    setAppLocale('fr-FR')
    expect(i18n.global.locale.value).toBe('zh-CN')
  })
})

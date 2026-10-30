import { i18n } from '../i18n'
import { ipcClient } from '../ipc/client'

/**
 * The UI language is a persisted setting (`language.locale`), but `createI18n` always
 * starts on zh-CN: without this the choice silently reverted on every restart.
 *
 * Called once before the app mounts, and again whenever the settings page changes it.
 */
export async function ensureLocale(): Promise<void> {
  try {
    const settings = await ipcClient.settingsGet()
    const saved = settings.language?.locale
    if (saved) setAppLocale(saved)
  } catch {
    // Mock / headless: keep the default
  }
}

/**
 * Applies a locale only when messages exist for it: the shipped bundles plus any
 * runtime-registered one (tests, future downloadable packs).
 */
export function setAppLocale(locale: string): void {
  const messages = i18n.global.messages.value as Record<string, unknown>
  if (!Object.prototype.hasOwnProperty.call(messages, locale)) return
  i18n.global.locale.value = locale
}

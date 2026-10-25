import { createI18n } from 'vue-i18n'

type MessageNode = { [key: string]: string | MessageNode }
type LocaleMessages = Record<string, Record<string, MessageNode>>

const modules = import.meta.glob<{ default: MessageNode }>('./locales/*/*.json', { eager: true })

const messages: LocaleMessages = {}

for (const [path, mod] of Object.entries(modules)) {
  const match = path.match(/\.\/locales\/([^/]+)\/([^/]+)\.json$/)
  if (!match) continue
  const [, locale, feature] = match
  if (!messages[locale]) {
    messages[locale] = {}
  }
  messages[locale][feature] = mod.default
}

export const availableLocales = Object.keys(messages).sort()

export const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'zh-CN',
  messages,
})

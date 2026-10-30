import { ref } from 'vue'
import { ipcClient } from '../ipc/client'

/**
 * Region format culture (`language.formatCulture`) used for dates and numbers.
 *
 * Loaded once at boot and updated from the settings page, so the choice applies to
 * every formatter without each component reading the settings store.
 */
const culture = ref('zh-CN')
let loading: Promise<void> | null = null

export async function ensureFormatCulture(): Promise<void> {
  if (loading) return loading
  loading = (async () => {
    try {
      const settings = await ipcClient.settingsGet()
      const saved = settings.language?.formatCulture
      if (saved) culture.value = saved
    } catch {
      // Keep the default in mock / headless environments
    }
  })()
  return loading
}

export function setFormatCulture(next: string): void {
  culture.value = next
}

export function useFormat() {
  function formatDate(iso: string | null | undefined): string {
    if (!iso) return ''
    const date = new Date(iso)
    if (Number.isNaN(date.getTime())) return ''
    try {
      return new Intl.DateTimeFormat(culture.value, {
        year: 'numeric',
        month: 'numeric',
        day: 'numeric',
      }).format(date)
    } catch {
      return date.toLocaleDateString()
    }
  }

  return { formatDate, culture }
}

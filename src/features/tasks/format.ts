import type { TaskState } from '../../ipc/client'

export function formatSpeed(bps: number): string {
  if (bps <= 0) return '0 B/s'
  const units = ['B/s', 'KB/s', 'MB/s', 'GB/s']
  let val = bps
  let u = 0
  while (val >= 1024 && u < units.length - 1) {
    val /= 1024
    u++
  }
  return `${val.toFixed(u === 0 ? 0 : 1)} ${units[u]}`
}

export function formatDuration(ms: number): string {
  if (ms <= 0) return '0s'
  const seconds = Math.floor(ms / 1000)
  if (seconds < 60) return `${seconds}s`
  const minutes = Math.floor(seconds / 60)
  const remSec = seconds % 60
  if (minutes < 60) return `${minutes}m ${remSec}s`
  const hours = Math.floor(minutes / 60)
  const remMin = minutes % 60
  return `${hours}h ${remMin}m`
}

export function resolveTaskTitle(
  key: string,
  params: Record<string, string>,
  t: (key: string, params?: Record<string, any>) => string,
  te: (key: string) => boolean,
): string {
  let lookupKey = key
  if (key.startsWith('task.')) {
    lookupKey = 'tasks.item.' + key.slice(5)
  }
  if (lookupKey === 'tasks.item.download.group' || key === 'task.download.group') {
    if (!params?.count) {
      const raw = te(lookupKey) ? t(lookupKey) : te(key) ? t(key) : ''
      if (raw) {
        return raw.replace(/\s*\([^)]*\)/g, '').trim()
      }
      return key
    }
  }
  if (te(lookupKey)) {
    return t(lookupKey, params)
  }
  if (te(key)) {
    return t(key, params)
  }
  return key
}

export function getTaskBadgeVariant(
  state: TaskState,
): 'primary' | 'success' | 'danger' | 'warning' | 'neutral' {
  switch (state) {
    case 'running':
      return 'primary'
    case 'success':
      return 'success'
    case 'failed':
      return 'danger'
    case 'retrying':
      return 'warning'
    case 'interrupted':
      return 'warning'
    case 'paused':
    case 'waiting':
    case 'canceled':
    default:
      return 'neutral'
  }
}

export function stepLabel(
  key: string,
  t: (key: string) => string,
  te: (key: string) => boolean,
): string {
  const i18nKey = `tasks.steps.${key}`
  return te(i18nKey) ? t(i18nKey) : key
}

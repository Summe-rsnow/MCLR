import { getCurrentScope, onScopeDispose, ref, shallowRef, type Ref, type ShallowRef } from 'vue'

export interface ResourceOptions<T> {
  /** Cache key shared across consumers; same key = same stale-while-revalidate entry. */
  key: string
  fetcher: (signal: AbortSignal) => Promise<T>
  /** Data younger than this is not refetched by `refresh()` (unless forced). */
  ttlMs?: number
}

export interface Resource<T> {
  data: ShallowRef<T | undefined>
  /** True only while the very first load is in flight (no data to show yet) — drives skeletons. */
  loading: Ref<boolean>
  /** True while a background refresh runs on top of already-visible data. */
  refreshing: Ref<boolean>
  error: ShallowRef<unknown>
  /** True once data exists (cached or fetched) or the first fetch settled with an error. */
  initialized: Ref<boolean>
  refresh: (force?: boolean) => Promise<void>
  /** Replace data locally (optimistic patch); also updates the shared cache. */
  set: (value: T) => void
  /** Drop the shared cache entry for this key. */
  invalidate: () => void
}

const cache = new Map<string, { data: unknown; at: number }>()

/** Test helper / logout hook: forget every cached resource. */
export function clearResourceCache(): void {
  cache.clear()
}

export function createResource<T>(opts: ResourceOptions<T>): Resource<T> {
  const ttlMs = opts.ttlMs ?? 10_000
  const cached = cache.get(opts.key) as { data: T; at: number } | undefined

  const data = shallowRef<T | undefined>(cached?.data)
  const loading = ref(false)
  const refreshing = ref(false)
  const error = shallowRef<unknown>(undefined)
  const initialized = ref(cached !== undefined)

  let requestId = 0
  let controller: AbortController | null = null

  async function refresh(force = false): Promise<void> {
    const entry = cache.get(opts.key)
    if (!force && entry && Date.now() - entry.at < ttlMs) {
      data.value = entry.data as T
      initialized.value = true
      return
    }
    controller?.abort()
    const ctl = new AbortController()
    controller = ctl
    const id = ++requestId
    const hasData = data.value !== undefined
    loading.value = !hasData
    refreshing.value = hasData
    try {
      const result = await opts.fetcher(ctl.signal)
      if (id !== requestId) return // a newer request owns the state now
      cache.set(opts.key, { data: result, at: Date.now() })
      data.value = result
      error.value = undefined
    } catch (e) {
      if (id !== requestId) return
      error.value = e
    } finally {
      if (id === requestId) {
        loading.value = false
        refreshing.value = false
        initialized.value = true
        controller = null
      }
    }
  }

  function set(value: T): void {
    data.value = value
    cache.set(opts.key, { data: value, at: Date.now() })
    initialized.value = true
  }

  function invalidate(): void {
    cache.delete(opts.key)
  }

  if (getCurrentScope()) {
    onScopeDispose(() => {
      requestId++ // drop any in-flight response
      controller?.abort()
    })
  }

  return { data, loading, refreshing, error, initialized, refresh, set, invalidate }
}

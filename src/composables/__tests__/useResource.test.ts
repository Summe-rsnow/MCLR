import { beforeEach, describe, expect, it, vi } from 'vitest'
import { clearResourceCache, createResource } from '../useResource'

function deferred<T>() {
  return Promise.withResolvers<T>()
}

describe('createResource', () => {
  beforeEach(() => clearResourceCache())

  it('shows loading only for the first load, then refreshing over existing data', async () => {
    const first = deferred<number>()
    const second = deferred<number>()
    const fetcher = vi.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise)
    const r = createResource({ key: 'a', fetcher, ttlMs: 0 })

    const p1 = r.refresh()
    expect(r.loading.value).toBe(true)
    expect(r.initialized.value).toBe(false)
    first.resolve(1)
    await p1
    expect(r.data.value).toBe(1)
    expect(r.loading.value).toBe(false)
    expect(r.initialized.value).toBe(true)

    const p2 = r.refresh()
    expect(r.loading.value).toBe(false)
    expect(r.refreshing.value).toBe(true)
    expect(r.data.value).toBe(1)
    second.resolve(2)
    await p2
    expect(r.data.value).toBe(2)
    expect(r.refreshing.value).toBe(false)
  })

  it('drops stale responses that resolve after a newer request', async () => {
    const slow = deferred<string>()
    const fast = deferred<string>()
    const fetcher = vi.fn().mockReturnValueOnce(slow.promise).mockReturnValueOnce(fast.promise)
    const r = createResource({ key: 'b', fetcher, ttlMs: 0 })

    const p1 = r.refresh(true)
    const p2 = r.refresh(true)
    fast.resolve('new')
    await p2
    slow.resolve('old')
    await p1
    expect(r.data.value).toBe('new')
  })

  it('serves cached data immediately and skips refetch within ttl', async () => {
    const fetcher = vi.fn().mockResolvedValue(['x'])
    const a = createResource({ key: 'c', fetcher, ttlMs: 60_000 })
    await a.refresh()
    expect(fetcher).toHaveBeenCalledTimes(1)

    const b = createResource({ key: 'c', fetcher, ttlMs: 60_000 })
    expect(b.data.value).toEqual(['x'])
    expect(b.initialized.value).toBe(true)
    await b.refresh()
    expect(fetcher).toHaveBeenCalledTimes(1)
    await b.refresh(true)
    expect(fetcher).toHaveBeenCalledTimes(2)
  })

  it('keeps stale data and records the error when a refresh fails', async () => {
    const fetcher = vi.fn().mockResolvedValueOnce(1).mockRejectedValueOnce(new Error('boom'))
    const r = createResource({ key: 'd', fetcher, ttlMs: 0 })
    await r.refresh()
    await r.refresh()
    expect(r.data.value).toBe(1)
    expect((r.error.value as Error).message).toBe('boom')
    expect(r.initialized.value).toBe(true)
  })

  it('marks initialized when the first fetch fails so skeletons stop', async () => {
    const r = createResource({ key: 'e', fetcher: () => Promise.reject(new Error('x')) })
    await r.refresh()
    expect(r.data.value).toBeUndefined()
    expect(r.loading.value).toBe(false)
    expect(r.initialized.value).toBe(true)
  })
})

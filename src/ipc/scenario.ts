/** Mock-IPC scenario (dev only): `demo` (default) ships demo data, `empty` makes every list command return `[]`. */
export const isEmptyScenario = import.meta.env.VITE_MOCK_SCENARIO?.trim() === 'empty'

export function isEmptyScenarioActive(): boolean {
  if (isEmptyScenario) return true
  if (typeof window === 'undefined') return false
  return new URLSearchParams(window.location.search).get('scenario') === 'empty'
}

/** Artificial latency for mock responses, e.g. `?latency=800` in the dev URL. */
export function mockLatencyMs(): number {
  if (typeof window === 'undefined') return 0
  const raw = new URLSearchParams(window.location.search).get('latency')
  const n = raw ? Number(raw) : 0
  return Number.isFinite(n) && n > 0 ? n : 0
}

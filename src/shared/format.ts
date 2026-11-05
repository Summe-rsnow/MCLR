const UNITS = ['KB', 'MB', 'GB', 'TB']

/**
 * Formats a byte count into a human-readable string.
 * Values < 1024 bytes format as `"N B"`.
 * Larger values use 1024-based binary units (KB, MB, GB, TB) with 1 decimal place below 100,
 * and 0 decimal places at or above 100 (e.g. `1.5 KB`, `150 MB`).
 */
export function formatBytes(n: number): string {
  if (n <= 0 || isNaN(n)) {
    return '0 B'
  }
  if (n < 1024) {
    return `${Math.floor(n)} B`
  }

  let val = n / 1024
  let unitIndex = 0

  while (val >= 1024 && unitIndex < UNITS.length - 1) {
    val /= 1024
    unitIndex++
  }

  const formatted = val >= 100 ? val.toFixed(0) : val.toFixed(1)
  return `${formatted} ${UNITS[unitIndex]}`
}

let cachedDisplacementMap: string | null | undefined

export function buildDisplacementMap(): string | null {
  if (cachedDisplacementMap !== undefined) {
    return cachedDisplacementMap
  }
  if (typeof document === 'undefined') {
    cachedDisplacementMap = null
    return null
  }
  try {
    const size = 256
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    if (!ctx || typeof ctx.createImageData !== 'function') {
      cachedDisplacementMap = null
      return null
    }
    const imageData = ctx.createImageData(size, size)
    const data = imageData.data

    for (let y = 0; y < size; y++) {
      const ny = (y / (size - 1)) * 2 - 1
      for (let x = 0; x < size; x++) {
        const nx = (x / (size - 1)) * 2 - 1
        const dist = Math.max(Math.abs(nx), Math.abs(ny))
        const t = Math.max(0, Math.min(1, (dist - 0.72) / (1 - 0.72)))
        const e = t * t * (3 - 2 * t)

        const r = Math.round(128 + 127 * (-nx * e))
        const g = Math.round(128 + 127 * (-ny * e))

        const idx = (y * size + x) * 4
        data[idx] = Math.max(0, Math.min(255, r))
        data[idx + 1] = Math.max(0, Math.min(255, g))
        data[idx + 2] = 128
        data[idx + 3] = 255
      }
    }

    ctx.putImageData(imageData, 0, 0)
    const url = canvas.toDataURL('image/png')
    if (!url || !url.startsWith('data:image/')) {
      cachedDisplacementMap = null
      return null
    }
    cachedDisplacementMap = url
    return url
  } catch {
    cachedDisplacementMap = null
    return null
  }
}

/**
 * Collision-aware positioning for viewport-fixed floating layers
 * (select dropdowns, tooltips, popovers, context menus).
 *
 * Every floating layer in `src/ui` positions with `position: fixed`, so the
 * numbers returned here are viewport coordinates and can be written straight
 * into `style.top` / `style.left`.
 *
 * Guarantees:
 * - the layer never leaves the viewport (flips to the opposite side when the
 *   preferred side has no room, and shifts along the cross axis);
 * - when the layer is genuinely too large for the viewport, `maxWidth` /
 *   `maxHeight` are returned so the caller can shrink instead of overflow.
 */

export type FloatingSide = 'top' | 'bottom'
export type FloatingAlign = 'start' | 'end' | 'center'

/** Distance kept from the viewport edge, in px. */
export const FLOATING_PADDING = 8

export interface FloatingSize {
  width: number
  height: number
}

export interface FloatingViewport {
  width: number
  height: number
}

export interface FloatingInput {
  /** Anchor rectangle, usually `element.getBoundingClientRect()`. */
  anchor: DOMRect | { top: number; bottom: number; left: number; right: number; width: number }
  /** Measured (or estimated) floating layer size. */
  size: FloatingSize
  /** Side to use when there is room for both. */
  preferred?: FloatingSide
  /** Cross-axis alignment relative to the anchor. */
  align?: FloatingAlign
  /** Gap between anchor and layer, in px. */
  offset?: number
  /** Minimum distance from the viewport edge, in px. */
  padding?: number
  viewport?: FloatingViewport
}

export interface FloatingResult {
  side: FloatingSide
  top: number
  left: number
  /** Set when the layer is wider than the available space. */
  maxWidth?: number
  /** Set when the layer is taller than the available space. */
  maxHeight?: number
}

function readViewport(explicit?: FloatingViewport): FloatingViewport {
  if (explicit) return explicit
  if (typeof window === 'undefined') return { width: 1280, height: 800 }
  return { width: window.innerWidth, height: window.innerHeight }
}

function clamp(value: number, min: number, max: number): number {
  const upper = Math.max(min, max)
  return Math.min(Math.max(value, min), upper)
}

/**
 * Computes the viewport-fixed position for a floating layer.
 *
 * Pure: no DOM reads except the fallback viewport size, so it is unit-testable
 * with explicit `anchor`, `size`, and `viewport`.
 */
export function computeFloatingPosition(input: FloatingInput): FloatingResult {
  const padding = input.padding ?? FLOATING_PADDING
  const offset = input.offset ?? 4
  const preferred = input.preferred ?? 'bottom'
  const align = input.align ?? 'start'
  const viewport = readViewport(input.viewport)
  const { anchor } = input

  // --- Main axis: flip when the preferred side does not fit ---------------
  const spaceBelow = viewport.height - anchor.bottom - offset - padding
  const spaceAbove = anchor.top - offset - padding
  const needed = input.size.height

  let side: FloatingSide = preferred
  if (preferred === 'bottom' && needed > spaceBelow) {
    if (needed <= spaceAbove || spaceAbove > spaceBelow) side = 'top'
  } else if (preferred === 'top' && needed > spaceAbove) {
    if (needed <= spaceBelow || spaceBelow > spaceAbove) side = 'bottom'
  }

  const available = side === 'bottom' ? spaceBelow : spaceAbove
  const maxHeight = needed > available && available > 0 ? available : undefined
  const height = maxHeight ?? needed

  const top =
    side === 'bottom' ? anchor.bottom + offset : Math.max(padding, anchor.top - offset - height)

  // --- Cross axis: align, then shift inside the viewport ------------------
  let left: number
  if (align === 'center') {
    left = anchor.left + anchor.width / 2 - input.size.width / 2
  } else if (align === 'end') {
    left = anchor.right - input.size.width
  } else {
    left = anchor.left
  }

  const usableWidth = viewport.width - padding * 2
  const maxWidth = input.size.width > usableWidth ? usableWidth : undefined
  const width = maxWidth ?? input.size.width
  left = clamp(left, padding, viewport.width - padding - width)

  return {
    side,
    top: Math.round(top),
    left: Math.round(left),
    ...(maxWidth !== undefined ? { maxWidth: Math.round(maxWidth) } : {}),
    ...(maxHeight !== undefined ? { maxHeight: Math.round(maxHeight) } : {}),
  }
}

/**
 * Positions an already-mounted floating element against its trigger.
 *
 * Writes inline styles directly (no reactive round-trip) so the element can be
 * re-measured cheaply on scroll and resize.
 */
export function applyFloatingPosition(
  floating: HTMLElement | null,
  trigger: HTMLElement | null,
  options: Omit<FloatingInput, 'anchor' | 'size'> & {
    /** Fixed size instead of measuring the element (defaults to measured). */
    size?: FloatingSize
    /** Also cap the element's own size, not only the returned coordinates. */
    constrain?: boolean
  } = {},
): FloatingResult | null {
  if (!floating || !trigger) return null

  const rect = trigger.getBoundingClientRect()
  const measured = options.size ?? {
    width: floating.offsetWidth,
    height: floating.offsetHeight,
  }
  const result = computeFloatingPosition({ ...options, anchor: rect, size: measured })

  floating.style.left = `${result.left}px`
  floating.style.top = `${result.top}px`
  floating.style.bottom = ''
  if (options.constrain !== false) {
    if (result.maxWidth !== undefined) floating.style.maxWidth = `${result.maxWidth}px`
    if (result.maxHeight !== undefined) floating.style.maxHeight = `${result.maxHeight}px`
  }
  floating.dataset.side = result.side

  return result
}

/**
 * Positions a pointer-anchored layer (context menu) so it stays on screen.
 * Returns the clamped top-left corner and the max size available at that point.
 */
export function computePointerPosition(
  point: { x: number; y: number },
  size: FloatingSize,
  options: { padding?: number; viewport?: FloatingViewport } = {},
): { left: number; top: number; maxWidth?: number; maxHeight?: number } {
  const padding = options.padding ?? FLOATING_PADDING
  const viewport = readViewport(options.viewport)
  const usableWidth = viewport.width - padding * 2
  const usableHeight = viewport.height - padding * 2
  const maxWidth = size.width > usableWidth ? usableWidth : undefined
  const maxHeight = size.height > usableHeight ? usableHeight : undefined
  const width = maxWidth ?? size.width
  const height = maxHeight ?? size.height

  return {
    left: Math.round(clamp(point.x, padding, viewport.width - padding - width)),
    top: Math.round(clamp(point.y, padding, viewport.height - padding - height)),
    ...(maxWidth !== undefined ? { maxWidth: Math.round(maxWidth) } : {}),
    ...(maxHeight !== undefined ? { maxHeight: Math.round(maxHeight) } : {}),
  }
}

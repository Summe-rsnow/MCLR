import { describe, it, expect } from 'vitest'
import { computeFloatingPosition, computePointerPosition, FLOATING_PADDING } from '../floating'

const VIEWPORT = { width: 1000, height: 600 }

function anchor(top: number, left: number, width = 120, height = 32) {
  return { top, bottom: top + height, left, right: left + width, width }
}

describe('computeFloatingPosition', () => {
  it('opens below and left-aligned when there is room', () => {
    const r = computeFloatingPosition({
      anchor: anchor(100, 100),
      size: { width: 200, height: 240 },
      viewport: VIEWPORT,
    })
    expect(r.side).toBe('bottom')
    expect(r.left).toBe(100)
    expect(r.top).toBe(136)
    expect(r.maxHeight).toBeUndefined()
  })

  it('flips above when the space below cannot fit the layer', () => {
    const r = computeFloatingPosition({
      anchor: anchor(520, 100),
      size: { width: 200, height: 240 },
      viewport: VIEWPORT,
    })
    expect(r.side).toBe('top')
    expect(r.top).toBe(520 - 4 - 240)
  })

  it('shifts left so a right-edge dropdown stays inside the viewport', () => {
    const r = computeFloatingPosition({
      anchor: anchor(100, 930),
      size: { width: 280, height: 200 },
      viewport: VIEWPORT,
    })
    expect(r.left).toBe(1000 - FLOATING_PADDING - 280)
    expect(r.left + 280).toBeLessThanOrEqual(1000 - FLOATING_PADDING)
  })

  it('shifts right for an anchor hanging off the left edge', () => {
    const r = computeFloatingPosition({
      anchor: anchor(100, -40),
      size: { width: 200, height: 100 },
      viewport: VIEWPORT,
    })
    expect(r.left).toBe(FLOATING_PADDING)
  })

  it('aligns to the anchor end edge on request', () => {
    const r = computeFloatingPosition({
      anchor: anchor(100, 400),
      size: { width: 200, height: 100 },
      align: 'end',
      viewport: VIEWPORT,
    })
    expect(r.left).toBe(320)
  })

  it('centers on the anchor and clamps a center alignment at the edge', () => {
    const centered = computeFloatingPosition({
      anchor: anchor(100, 400, 100),
      size: { width: 60, height: 40 },
      align: 'center',
      viewport: VIEWPORT,
    })
    expect(centered.left).toBe(420)

    const clamped = computeFloatingPosition({
      anchor: anchor(100, 960, 30),
      size: { width: 60, height: 40 },
      align: 'center',
      viewport: VIEWPORT,
    })
    expect(clamped.left).toBe(1000 - FLOATING_PADDING - 60)
  })

  it('caps maxWidth when the layer is wider than the viewport', () => {
    const r = computeFloatingPosition({
      anchor: anchor(100, 500),
      size: { width: 4000, height: 100 },
      viewport: VIEWPORT,
    })
    expect(r.maxWidth).toBe(1000 - FLOATING_PADDING * 2)
    expect(r.left).toBe(FLOATING_PADDING)
  })

  it('caps maxHeight instead of flipping when neither side fits', () => {
    const r = computeFloatingPosition({
      anchor: anchor(280, 100),
      size: { width: 200, height: 500 },
      viewport: { width: 1000, height: 600 },
    })
    expect(r.maxHeight).toBeDefined()
    expect(r.top + (r.maxHeight ?? 0)).toBeLessThanOrEqual(600)
  })
})

describe('computePointerPosition', () => {
  it('keeps the pointer position when there is room', () => {
    const r = computePointerPosition(
      { x: 100, y: 100 },
      { width: 160, height: 120 },
      { viewport: VIEWPORT },
    )
    expect(r).toMatchObject({ left: 100, top: 100 })
  })

  it('pulls a right-click menu back inside the viewport', () => {
    const r = computePointerPosition(
      { x: 990, y: 590 },
      { width: 160, height: 120 },
      { viewport: VIEWPORT },
    )
    expect(r.left).toBe(1000 - FLOATING_PADDING - 160)
    expect(r.top).toBe(600 - FLOATING_PADDING - 120)
  })
})

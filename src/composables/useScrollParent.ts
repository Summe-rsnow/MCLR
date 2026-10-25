/** Nearest ancestor that scrolls vertically (`overflow-y: auto|scroll|overlay`), or null. */
export function findScrollParent(el: HTMLElement | null): HTMLElement | null {
  let node = el?.parentElement ?? null
  while (node && node !== document.body) {
    const oy = getComputedStyle(node).overflowY
    if (oy === 'auto' || oy === 'scroll' || oy === 'overlay') return node
    node = node.parentElement
  }
  return null
}

/** Vertical offset of `el` inside the scrollable `scroller` content (for virtualizer scrollMargin). */
export function offsetWithin(el: HTMLElement, scroller: HTMLElement): number {
  return el.getBoundingClientRect().top - scroller.getBoundingClientRect().top + scroller.scrollTop
}

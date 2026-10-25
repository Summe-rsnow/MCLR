export interface OverlayStackItem {
  id: number
  close: () => void
  readonly closeOnEsc: boolean
}

let nextId = 0
export const overlayStack: OverlayStackItem[] = []
let bodyLockedCount = 0
let originalOverflow: string | null = null

function handleGlobalKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape' && overlayStack.length > 0) {
    const top = overlayStack[overlayStack.length - 1]
    if (top && top.closeOnEsc) {
      e.preventDefault()
      e.stopPropagation()
      top.close()
    }
  }
}

export function lockBody() {
  if (typeof document === 'undefined') return
  if (bodyLockedCount === 0) {
    originalOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
    if (typeof window !== 'undefined') {
      window.addEventListener('keydown', handleGlobalKeyDown)
    }
  }
  bodyLockedCount++
}

export function unlockBody() {
  if (typeof document === 'undefined') return
  bodyLockedCount = Math.max(0, bodyLockedCount - 1)
  if (bodyLockedCount === 0) {
    if (originalOverflow !== null && originalOverflow !== '') {
      document.body.style.overflow = originalOverflow
    } else {
      document.body.style.removeProperty('overflow')
    }
    originalOverflow = null
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', handleGlobalKeyDown)
    }
  }
}

export function getNextOverlayId(): number {
  return ++nextId
}

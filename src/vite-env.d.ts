/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<Record<string, unknown>, Record<string, unknown>, unknown>
  export default component
}

declare module 'culori' {
  export interface Oklch {
    mode: 'oklch'
    l: number
    c: number
    h?: number
    alpha?: number
  }

  export function oklch(color: string | unknown): Oklch | undefined
  export function clampChroma(color: unknown, mode?: string): unknown
  export function formatRgb(color: unknown): string
}

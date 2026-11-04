import type { Component } from 'vue'

export type TabGroupKey = 'game' | 'resources'

export interface InstanceDetailTab {
  key: string
  titleKey: string
  icon: string
  order: number
  group?: TabGroupKey
  component: Component
}

// Auto-discovered instance detail sub-tabs
const tabModules = import.meta.glob<{ default: InstanceDetailTab }>('./tabs/*.tab.ts', {
  eager: true,
})

export const instanceDetailTabs: InstanceDetailTab[] = Object.values(tabModules)
  .map((mod) => mod.default)
  .filter((tab): tab is InstanceDetailTab => Boolean(tab))
  .sort((a, b) => a.order - b.order)

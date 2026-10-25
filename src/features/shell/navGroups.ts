export const NAV_GROUPS = [
  { key: 'start', titleKey: 'shell.navGroup.start' },
  { key: 'game', titleKey: 'shell.navGroup.game' },
  { key: 'discover', titleKey: 'shell.navGroup.discover' },
  { key: 'system', titleKey: 'shell.navGroup.system' },
  { key: 'dev', titleKey: 'shell.navGroup.dev' },
] as const

export type NavGroupKey = (typeof NAV_GROUPS)[number]['key']

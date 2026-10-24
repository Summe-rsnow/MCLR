import ModsTab from './ModsTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'mods',
  titleKey: 'instanceDetail.tabs.mods',
  icon: 'i-lucide-puzzle',
  order: 30,
  component: ModsTab,
}

export default tab

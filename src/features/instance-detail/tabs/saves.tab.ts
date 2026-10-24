import SavesTab from './SavesTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'saves',
  titleKey: 'instanceDetail.tabs.saves',
  icon: 'i-lucide-globe',
  order: 70,
  component: SavesTab,
}

export default tab

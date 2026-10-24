import ResourcePacksTab from './ResourcePacksTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'resourcepacks',
  titleKey: 'instanceDetail.tabs.resourcepacks',
  icon: 'i-lucide-palette',
  order: 40,
  component: ResourcePacksTab,
}

export default tab

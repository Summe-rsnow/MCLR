import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'resourcepacks',
  titleKey: 'instanceDetail.tabs.resourcepacks',
  icon: 'i-lucide-palette',
  order: 80,
  component: defineAsyncComponent(() => import('./ResourcePacksTab.vue')),
}

export default tab

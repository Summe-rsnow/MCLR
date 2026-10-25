import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'mods',
  titleKey: 'instanceDetail.tabs.mods',
  icon: 'i-lucide-puzzle',
  order: 70,
  component: defineAsyncComponent(() => import('./ModsTab.vue')),
}

export default tab

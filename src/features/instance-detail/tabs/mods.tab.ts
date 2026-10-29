import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'mods',
  titleKey: 'instanceDetail.tabs.mods',
  icon: 'i-lucide-puzzle',
  order: 45,
  group: 'resources',
  component: defineAsyncComponent(() => import('./ModsTab.vue')),
}

export default tab

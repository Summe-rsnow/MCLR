import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'saves',
  titleKey: 'instanceDetail.tabs.saves',
  icon: 'i-lucide-globe',
  order: 50,
  group: 'resources',
  component: defineAsyncComponent(() => import('./SavesTab.vue')),
}

export default tab

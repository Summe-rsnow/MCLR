import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'schematics',
  titleKey: 'instanceDetail.tabs.schematics',
  icon: 'i-lucide-box',
  order: 100,
  group: 'resources',
  component: defineAsyncComponent(() => import('./SchematicsTab.vue')),
}

export default tab

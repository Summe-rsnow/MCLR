import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'export',
  titleKey: 'instanceDetail.tabs.export',
  icon: 'i-lucide-package-plus',
  order: 40,
  group: 'game',
  component: defineAsyncComponent(() => import('./ExportTab.vue')),
}

export default tab

import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'servers',
  titleKey: 'instanceDetail.tabs.servers',
  icon: 'i-lucide-server',
  order: 110,
  group: 'resources',
  component: defineAsyncComponent(() => import('./ServersTab.vue')),
}

export default tab

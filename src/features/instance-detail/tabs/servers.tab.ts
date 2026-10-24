import ServersTab from './ServersTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'servers',
  titleKey: 'instanceDetail.tabs.servers',
  icon: 'i-lucide-server',
  order: 90,
  component: ServersTab,
}

export default tab

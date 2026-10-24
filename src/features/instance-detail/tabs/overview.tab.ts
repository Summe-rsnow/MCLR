import OverviewTab from './OverviewTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'overview',
  titleKey: 'instanceDetail.tabs.overview',
  icon: 'i-lucide-info',
  order: 10,
  component: OverviewTab,
}

export default tab

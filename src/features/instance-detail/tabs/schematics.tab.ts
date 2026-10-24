import SchematicsTab from './SchematicsTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'schematics',
  titleKey: 'instanceDetail.tabs.schematics',
  icon: 'i-lucide-box-select',
  order: 60,
  component: SchematicsTab,
}

export default tab

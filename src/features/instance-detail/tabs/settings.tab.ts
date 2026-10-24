import SettingsTab from './SettingsTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'settings',
  titleKey: 'instanceDetail.tabs.settings',
  icon: 'i-lucide-sliders-horizontal',
  order: 20,
  component: SettingsTab,
}

export default tab

import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'settings',
  titleKey: 'instanceDetail.tabs.settings',
  icon: 'i-lucide-sliders-horizontal',
  order: 20,
  component: defineAsyncComponent(() => import('./SettingsTab.vue')),
}

export default tab

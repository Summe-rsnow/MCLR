import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'presets',
  titleKey: 'instanceDetail.tabs.presets',
  icon: 'i-lucide-package-check',
  order: 46,
  group: 'resources',
  component: defineAsyncComponent(() => import('./PresetsTab.vue')),
}

export default tab

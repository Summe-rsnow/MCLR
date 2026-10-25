import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'screenshots',
  titleKey: 'instanceDetail.tabs.screenshots',
  icon: 'i-lucide-image',
  order: 60,
  component: defineAsyncComponent(() => import('./ScreenshotsTab.vue')),
}

export default tab

import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'install',
  titleKey: 'download.modify.tabTitle',
  icon: 'i-lucide-wrench',
  order: 15,
  component: defineAsyncComponent(() => import('./InstallTab.vue')),
}

export default tab

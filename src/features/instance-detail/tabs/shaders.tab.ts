import { defineAsyncComponent } from 'vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'shaders',
  titleKey: 'instanceDetail.tabs.shaders',
  icon: 'i-lucide-sparkles',
  order: 90,
  component: defineAsyncComponent(() => import('./ShadersTab.vue')),
}

export default tab

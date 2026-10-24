import ShadersTab from './ShadersTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'shaders',
  titleKey: 'instanceDetail.tabs.shaders',
  icon: 'i-lucide-sparkles',
  order: 50,
  component: ShadersTab,
}

export default tab

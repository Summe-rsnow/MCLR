import ExportTab from './ExportTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'export',
  titleKey: 'instanceDetail.tabs.export',
  icon: 'i-lucide-package-plus',
  order: 100,
  component: ExportTab,
}

export default tab

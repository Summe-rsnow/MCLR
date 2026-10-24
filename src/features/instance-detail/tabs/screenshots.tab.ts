import ScreenshotsTab from './ScreenshotsTab.vue'
import type { InstanceDetailTab } from '../tabs'

const tab: InstanceDetailTab = {
  key: 'screenshots',
  titleKey: 'instanceDetail.tabs.screenshots',
  icon: 'i-lucide-camera',
  order: 80,
  component: ScreenshotsTab,
}

export default tab

<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useInstancesStore } from '../instances/store'
import { useLaunchStore } from '../launch/store'
import { instanceDetailTabs, type TabGroupKey } from './tabs'
import ModsDisabledTab from './tabs/ModsDisabledTab.vue'
import {
  MPageHeader,
  MButton,
  MBadge,
  MSegmented,
  MTabs,
  MCard,
  MEmpty,
  MSkeleton,
  MLoaderIcon,
} from '../../ui'
const props = defineProps<{
  id?: string
}>()

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const instancesStore = useInstancesStore()
const launchStore = useLaunchStore()

const currentInstanceId = computed(() => {
  return (props.id || (route.params.id as string) || '').trim()
})

const instance = computed(() => {
  return instancesStore.instances.find((i) => i.id === currentInstanceId.value)
})

const activeTab = ref(instanceDetailTabs[0]?.key || 'overview')
const direction = ref<'forward' | 'back'>('forward')

const tabGroupMap = computed(() => {
  const map = new Map<string, TabGroupKey>()
  for (const tab of instanceDetailTabs) {
    map.set(tab.key, tab.group || 'game')
  }
  return map
})

const activeGroup = ref<TabGroupKey>(tabGroupMap.value.get(activeTab.value) || 'game')

const groupOptions = computed(() => [
  {
    label: t('instanceDetail.groups.game'),
    value: 'game',
    icon: 'i-lucide-gamepad-2',
  },
  {
    label: t('instanceDetail.groups.resources'),
    value: 'resources',
    icon: 'i-lucide-folder-archive',
  },
])

const visibleTabs = computed(() => {
  return instanceDetailTabs.filter((tab) => (tab.group || 'game') === activeGroup.value)
})

const visibleTabOptions = computed(() =>
  visibleTabs.value.map((tab) => ({
    key: tab.key,
    label: t(tab.titleKey),
    icon: tab.icon,
  })),
)

// Keep activeGroup synchronized when activeTab changes
watch(
  activeTab,
  (tab) => {
    const group = tabGroupMap.value.get(tab)
    if (group && group !== activeGroup.value) {
      activeGroup.value = group
    }
  },
  { immediate: true },
)

function onGroupChange(group: string) {
  const newGroup = group as TabGroupKey
  activeGroup.value = newGroup
  const currentTabInGroup = visibleTabs.value.some((t) => t.key === activeTab.value)
  if (!currentTabInGroup) {
    const firstTab = visibleTabs.value[0]
    if (firstTab) {
      onTabChange(firstTab.key)
    }
  }
}

// Sync with query param if present
watch(
  () => route.query.tab,
  (tab) => {
    if (
      tab &&
      typeof tab === 'string' &&
      tab !== activeTab.value &&
      instanceDetailTabs.some((t) => t.key === tab)
    ) {
      const oldOrder = instanceDetailTabs.find((t) => t.key === activeTab.value)?.order ?? 0
      const newOrder = instanceDetailTabs.find((t) => t.key === tab)?.order ?? 0
      direction.value = newOrder >= oldOrder ? 'forward' : 'back'
      activeTab.value = tab
    }
  },
  { immediate: true },
)

function onTabChange(key: string) {
  const oldOrder = instanceDetailTabs.find((t) => t.key === activeTab.value)?.order ?? 0
  const newOrder = instanceDetailTabs.find((t) => t.key === key)?.order ?? 0
  direction.value = newOrder >= oldOrder ? 'forward' : 'back'
  activeTab.value = key
  router.replace({
    query: {
      ...route.query,
      tab: key,
    },
  })
}

const activeComponent = computed(() => {
  const target = instanceDetailTabs.find((tab) => tab.key === activeTab.value)
  return target?.component || null
})

onMounted(async () => {
  if (instancesStore.instances.length === 0) {
    await instancesStore.loadSettings()
    await instancesStore.fetchInstances()
  }
})

function goBack() {
  router.push('/instances')
}

function handleLaunch() {
  if (!instance.value) return
  launchStore.selectInstance(instance.value.id)
  router.push('/launch')
}
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="instance ? instance.name : currentInstanceId || t('instanceDetail.nav')"
      :description="
        instance?.mcVersion
          ? `${instance.mcVersion} • ${instance.loaders.length > 0 ? instance.loaders.join(', ') : 'Vanilla'}`
          : undefined
      "
    >
      <template #icon>
        <MLoaderIcon
          :kind="instance?.kind || (instance?.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
          :icon="instance?.config?.icon"
          :size="40"
          class="shrink-0"
        />
      </template>
      <template #actions>
        <div class="flex items-center gap-2">
          <MButton variant="text" size="sm" icon="i-lucide-arrow-left" @click="goBack">
            {{ t('instanceDetail.back') }}
          </MButton>
          <MButton
            v-if="instance"
            variant="primary"
            size="sm"
            icon="i-lucide-play"
            @click="handleLaunch"
          >
            {{ t('instanceDetail.launch') }}
          </MButton>
        </div>
      </template>
    </MPageHeader>

    <!-- Header Summary Card Skeleton (when loading) -->
    <MCard
      v-if="!instance && instancesStore.loading"
      padding="md"
      class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
    >
      <div class="flex items-center gap-4 min-w-0">
        <MSkeleton width="3.5rem" height="3.5rem" class="rounded-xl shrink-0" />
        <div class="min-w-0 space-y-2">
          <MSkeleton width="12rem" height="1.5rem" />
          <div class="flex items-center gap-2">
            <MSkeleton width="4rem" height="1.25rem" class="rounded-md" />
            <MSkeleton width="4rem" height="1.25rem" class="rounded-md" />
          </div>
        </div>
      </div>
      <div class="flex items-center gap-2 self-end sm:self-auto">
        <MSkeleton width="6rem" height="2.25rem" class="rounded-lg" />
        <MSkeleton width="6rem" height="2.25rem" class="rounded-lg" />
      </div>
    </MCard>

    <!-- Header Summary Card -->
    <MCard
      v-else-if="instance"
      padding="md"
      class="flex flex-col sm:flex-row sm:items-center justify-between gap-4"
    >
      <div class="flex items-center gap-4 min-w-0">
        <MLoaderIcon
          :kind="instance.kind || (instance.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
          :icon="instance.config?.icon"
          :size="56"
          class="shrink-0"
        />
        <div class="min-w-0 space-y-1">
          <div class="flex items-center gap-2">
            <h2 class="text-xl font-bold text-fg truncate">
              {{ instance.name }}
            </h2>
            <MBadge v-if="instance.state !== 'ok'" variant="danger">
              {{ t(`instances.card.state.${instance.state}`) }}
            </MBadge>
          </div>
          <div class="flex flex-wrap items-center gap-2 select-text">
            <MBadge variant="primary">
              {{ instance.mcVersion }}
            </MBadge>
            <MBadge v-for="loader in instance.loaders" :key="loader" variant="neutral">
              {{ loader }}
            </MBadge>
            <MBadge v-if="instance.loaders.length === 0" variant="neutral"> Vanilla </MBadge>
          </div>
        </div>
      </div>
    </MCard>
    <!-- Navigation Tabs with Group Switcher -->
    <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
      <MSegmented
        :model-value="activeGroup"
        :options="groupOptions"
        size="sm"
        class="shrink-0"
        @update:model-value="onGroupChange"
      />
      <div class="min-w-0 flex-1">
        <MTabs
          :model-value="activeTab"
          :tabs="visibleTabOptions"
          @update:model-value="onTabChange"
        />
      </div>
    </div>
    <!-- Tab Content -->
    <div class="max-w-full overflow-x-hidden">
      <div v-if="!instance && instancesStore.loading" class="space-y-4">
        <div class="flex items-center gap-2 py-2">
          <MSkeleton v-for="n in 5" :key="n" width="5.5rem" height="2rem" class="rounded-lg" />
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <MSkeleton height="14rem" class="rounded-2xl" />
          <MSkeleton height="14rem" class="rounded-2xl" />
        </div>
      </div>
      <div v-else-if="!instance" class="py-20">
        <MEmpty :title="t('instanceDetail.notFound')" icon="i-lucide-circle-alert">
          <MButton variant="tonal" size="sm" icon="i-lucide-arrow-left" @click="goBack">
            {{ t('instanceDetail.back') }}
          </MButton>
        </MEmpty>
      </div>
      <div v-else class="max-w-full overflow-x-hidden">
        <Transition :name="direction === 'forward' ? 'm-tab-forward' : 'm-tab-back'" mode="out-in">
          <div :key="activeTab" class="max-w-full overflow-x-hidden">
            <ModsDisabledTab v-if="activeTab === 'mods' && instance && !instance.modable" />
            <component
              :is="activeComponent"
              v-else
              :instance="instance"
              :instance-id="instance.id"
            />
          </div>
        </Transition>
      </div>
    </div>
  </div>
</template>

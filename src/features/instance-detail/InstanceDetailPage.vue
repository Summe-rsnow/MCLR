<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useInstancesStore } from '../instances/store'
import { instanceDetailTabs } from './tabs'
import {
  MButton,
  MBadge,
  MTabs,
  MEmpty,
  MSkeleton,
} from '../../ui'

const props = defineProps<{
  id?: string
}>()

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const instancesStore = useInstancesStore()

const currentInstanceId = computed(() => {
  return (props.id || (route.params.id as string) || '').trim()
})

const instance = computed(() => {
  return instancesStore.instances.find((i) => i.id === currentInstanceId.value)
})

const activeTab = ref(instanceDetailTabs[0]?.key || 'overview')

// Sync with query param if present
watch(
  () => route.query.tab,
  (tab) => {
    if (tab && typeof tab === 'string' && instanceDetailTabs.some((t) => t.key === tab)) {
      activeTab.value = tab
    }
  },
  { immediate: true },
)

function onTabChange(key: string) {
  activeTab.value = key
  router.replace({
    query: {
      ...route.query,
      tab: key,
    },
  })
}

const tabOptions = computed(() =>
  instanceDetailTabs.map((tab) => ({
    key: tab.key,
    label: t(tab.titleKey),
    icon: tab.icon,
  })),
)

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
</script>

<template>
  <div class="h-full flex flex-col overflow-hidden bg-surface">
    <!-- Header -->
    <div class="shrink-0 border-b border-outline/20 px-6 py-4 flex items-center justify-between gap-4 bg-surface/80 backdrop-blur">
      <div class="flex items-center gap-3 min-w-0">
        <MButton variant="text" size="sm" @click="goBack">
          <span class="w-4 h-4 i-lucide-arrow-left mr-1" />
          {{ t('instanceDetail.back') }}
        </MButton>
        <div class="h-4 w-px bg-outline/20" />
        <div class="flex items-center gap-2.5 min-w-0">
          <div class="w-8 h-8 rounded-lg bg-surfaceVariant/60 flex items-center justify-center shrink-0 border border-outline/30 text-primary">
            <span
              class="w-5 h-5"
              :class="{
                'i-lucide-box': instance?.kind === 'vanilla',
                'i-lucide-layers': instance?.kind === 'fabric',
                'i-lucide-flame': instance?.kind === 'forge' || instance?.kind === 'neoforge',
                'i-lucide-cpu': instance?.kind !== 'vanilla' && instance?.kind !== 'fabric' && instance?.kind !== 'forge' && instance?.kind !== 'neoforge',
              }"
            />
          </div>
          <div class="min-w-0">
            <h1 class="text-base font-bold text-text truncate">
              {{ instance ? instance.name : currentInstanceId }}
            </h1>
            <div class="flex items-center gap-2 text-xs text-textMuted">
              <span class="font-mono">{{ instance?.mcVersion || '' }}</span>
              <MBadge v-if="instance && instance.state !== 'ok'" variant="danger">
                {{ t(`instances.card.state.${instance.state}`) }}
              </MBadge>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Navigation Tabs -->
    <div class="shrink-0 px-6 pt-3 border-b border-outline/20 bg-surface/50">
      <MTabs
        :model-value="activeTab"
        :tabs="tabOptions"
        @update:model-value="onTabChange"
      />
    </div>

    <!-- Tab Content -->
    <div class="flex-1 overflow-y-auto p-6">
      <div v-if="!instance && instancesStore.loading" class="space-y-4">
        <MSkeleton class="h-10 w-48" />
        <MSkeleton class="h-64 w-full" />
      </div>
      <div v-else-if="!instance" class="py-20">
        <MEmpty
          :title="t('instanceDetail.notFound')"
          icon="i-lucide-alert-circle"
        />
      </div>
      <component
        :is="activeComponent"
        v-else
        :key="activeTab"
        :instance="instance"
        :instance-id="instance.id"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { InstanceInfo } from '../../ipc/client'
import { useInstancesStore } from '../instances/store'
import { MCard, MBadge, MLoaderIcon, MIconButton } from '../../ui'

const props = defineProps<{
  instance: InstanceInfo
}>()

const emit = defineEmits<{
  (e: 'contextmenu', ev: MouseEvent, instance: InstanceInfo): void
}>()

const { t } = useI18n()
const router = useRouter()
const instancesStore = useInstancesStore()

const isStarred = computed(() => {
  return Boolean(
    (props.instance as unknown as { starred?: boolean }).starred ||
    props.instance.config?.starred ||
    props.instance.config?.pinned,
  )
})

const instanceLoaderKind = computed(() => {
  return props.instance.kind || (props.instance.loaders?.[0]?.toLowerCase() ?? 'vanilla')
})

function onOpenFolder() {
  instancesStore.openFolder(props.instance.id)
}

function onManageInstance() {
  router.push('/instances/' + encodeURIComponent(props.instance.id))
}
</script>

<template>
  <MCard
    variant="well"
    padding="sm"
    class="flex items-center justify-between gap-3"
    data-testid="instance-preview-card"
    @contextmenu="emit('contextmenu', $event, instance)"
  >
    <!-- Left: Loader / Instance Icon -->
    <MLoaderIcon :kind="instanceLoaderKind" :icon="instance.config?.icon" :size="40" />

    <!-- Center: Details -->
    <div class="min-w-0 flex-1">
      <!-- Row 1: Name + Star -->
      <div class="flex items-center gap-1.5">
        <span class="font-semibold text-sm text-fg truncate">
          {{ instance.name }}
        </span>
        <span
          v-if="isStarred"
          class="i-lucide-star text-warning w-3.5 h-3.5 shrink-0"
          data-testid="instance-starred-icon"
        />
      </div>

      <!-- Row 2: Badges -->
      <div class="flex flex-wrap items-center gap-1.5 mt-0.5 text-xs text-fg-muted">
        <MBadge variant="primary" size="sm">
          {{ instance.mcVersion || 'Unknown' }}
        </MBadge>
        <template v-if="instance.loaders && instance.loaders.length > 0">
          <MBadge v-for="loader in instance.loaders" :key="loader" variant="neutral" size="sm">
            {{ loader }}
          </MBadge>
        </template>
        <MBadge v-else variant="neutral" size="sm">
          {{ t('instances.filter.vanilla') }}
        </MBadge>
        <MBadge v-if="instance.config?.isolation === 'on'" variant="info" size="sm">
          {{ t('instances.actions.isolationOn') }}
        </MBadge>
        <MBadge v-if="instance.state && instance.state !== 'ok'" variant="danger" size="sm">
          {{ t('instances.card.state.' + instance.state) }}
        </MBadge>
      </div>
    </div>

    <!-- Right: Actions -->
    <div class="flex items-center gap-1 shrink-0">
      <MIconButton
        icon="i-lucide-folder-open"
        size="sm"
        variant="text"
        :tooltip="t('instances.actions.openFolder')"
        :title="t('instances.actions.openFolder')"
        data-testid="instance-open-folder-btn"
        @click="onOpenFolder"
      />
      <MIconButton
        icon="i-lucide-sliders-horizontal"
        size="sm"
        variant="text"
        :tooltip="t('instances.actions.manage')"
        :title="t('instances.actions.manage')"
        data-testid="instance-manage-btn"
        @click="onManageInstance"
      />
    </div>
  </MCard>
</template>

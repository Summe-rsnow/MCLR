<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { DatapackInfo, SaveInfo } from '../../../ipc/client'
import { MDrawer, MButton, MBadge, MSwitch, MEmpty, MSkeleton } from '../../../ui'

defineProps<{
  open: boolean
  save: SaveInfo | null
  datapacks: DatapackInfo[]
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'toggle', pack: DatapackInfo, enabled: boolean): void
}>()

const { t } = useI18n()
</script>

<template>
  <MDrawer
    :open="open"
    placement="right"
    size="md"
    :title="
      t('instanceDetail.saves.datapackModalTitle', { name: save?.name || save?.folderName || '' })
    "
    @update:open="emit('update:open', $event)"
  >
    <div v-if="loading" class="space-y-3 py-2">
      <MSkeleton v-for="n in 3" :key="n" class="h-16 rounded-xl" />
    </div>

    <div v-else-if="datapacks.length === 0" class="py-12">
      <MEmpty :title="t('instanceDetail.saves.datapackEmpty')" icon="i-lucide-layers" />
    </div>

    <div v-else class="space-y-2 py-1">
      <div
        v-for="pack in datapacks"
        :key="pack.id"
        class="p-3 rounded-xl border border-line bg-raised flex items-center justify-between gap-3"
      >
        <div class="min-w-0">
          <div class="text-sm font-semibold text-fg flex items-center gap-2 truncate">
            <span>{{ pack.fileName }}</span>
            <MBadge v-if="pack.builtIn" variant="neutral" size="sm">
              {{ t('instanceDetail.saves.builtIn') }}
            </MBadge>
          </div>
          <div v-if="pack.description" class="text-xs text-fg-muted mt-0.5 truncate">
            {{ pack.description }}
          </div>
          <div class="text-[11px] text-fg-muted/60 font-mono mt-0.5">
            {{ t('instanceDetail.saves.datapackFormat') }}: {{ pack.packFormat }}
          </div>
        </div>

        <div class="shrink-0">
          <MSwitch
            :model-value="pack.enabled"
            :disabled="pack.builtIn"
            @update:model-value="emit('toggle', pack, $event)"
          />
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex justify-end">
        <MButton variant="tonal" @click="emit('update:open', false)">
          {{ t('common.close') }}
        </MButton>
      </div>
    </template>
  </MDrawer>
</template>

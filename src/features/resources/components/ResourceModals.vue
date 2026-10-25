<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { ResourceEntry, SchematicInfo } from '../../../ipc/client'
import { MDialog, MButton, MSkeleton } from '../../../ui'
import JarInJarDrawer from '../JarInJarDrawer.vue'

defineProps<{
  instanceId: string
  jijOpen: boolean
  jijMod: ResourceEntry | null
  schematicOpen: boolean
  inspectingSchematic: boolean
  schematicDetail: SchematicInfo | null
  cascadeOpen: boolean
  cascadeAction: 'disable' | 'delete'
  cascadeDependents: string[]
  updateOpen: boolean
  updateTarget: ResourceEntry | null
  batchUpdateRiskOpen: boolean
  pendingBatchUpdates: ResourceEntry[]
}>()

const emit = defineEmits<{
  (e: 'update:jijOpen', val: boolean): void
  (e: 'update:schematicOpen', val: boolean): void
  (e: 'update:cascadeOpen', val: boolean): void
  (e: 'update:updateOpen', val: boolean): void
  (e: 'update:batchUpdateRiskOpen', val: boolean): void
  (e: 'cascadeConfirm'): void
  (e: 'singleUpdate', entry: ResourceEntry): void
  (e: 'batchUpdate', list: ResourceEntry[]): void
}>()

const { t } = useI18n()
</script>

<template>
  <div>
    <!-- Jar-in-Jar Drawer -->
    <JarInJarDrawer
      :model-value="jijOpen"
      :instance-id="instanceId"
      :mod="jijMod"
      @update:model-value="emit('update:jijOpen', $event)"
    />

    <!-- Schematic Inspection Dialog -->
    <MDialog
      :open="schematicOpen"
      :title="t('resources.actions.info')"
      class="max-w-md"
      @update:open="emit('update:schematicOpen', $event)"
      @cancel="emit('update:schematicOpen', false)"
    >
      <div v-if="inspectingSchematic" class="space-y-2 py-4">
        <MSkeleton class="h-6 rounded" />
        <MSkeleton class="h-6 rounded" />
        <MSkeleton class="h-6 rounded" />
      </div>
      <div v-else-if="schematicDetail" class="space-y-3 py-2 text-xs">
        <div class="flex justify-between border-b border-line pb-1.5">
          <span class="text-fg-muted">{{ t('resources.schematic.dimensions') }}</span>
          <span class="font-mono text-fg font-medium">
            {{ schematicDetail.size[0] }} × {{ schematicDetail.size[1] }} ×
            {{ schematicDetail.size[2] }}
          </span>
        </div>
        <div class="flex justify-between border-b border-line pb-1.5">
          <span class="text-fg-muted">{{ t('resources.schematic.volume') }}</span>
          <span class="font-mono text-fg font-medium">
            {{ schematicDetail.totalVolume.toLocaleString() }}
          </span>
        </div>
        <div class="flex justify-between border-b border-line pb-1.5">
          <span class="text-fg-muted">{{ t('resources.schematic.blocks') }}</span>
          <span class="font-mono text-fg font-medium">
            {{ schematicDetail.totalBlocks.toLocaleString() }}
          </span>
        </div>
        <div class="flex justify-between border-b border-line pb-1.5">
          <span class="text-fg-muted">{{ t('resources.schematic.regions') }}</span>
          <span class="font-mono text-fg font-medium">
            {{ schematicDetail.regionCount }}
          </span>
        </div>
        <div v-if="schematicDetail.author" class="flex justify-between border-b border-line pb-1.5">
          <span class="text-fg-muted">{{ t('resources.schematic.author') }}</span>
          <span class="text-fg font-medium">{{ schematicDetail.author }}</span>
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end">
          <MButton variant="tonal" @click="emit('update:schematicOpen', false)">
            {{ t('resources.actions.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Cascade Prompt Dialog -->
    <MDialog
      :open="cascadeOpen"
      :title="t('resources.cascade.title')"
      class="max-w-md"
      @update:open="emit('update:cascadeOpen', $event)"
      @cancel="emit('update:cascadeOpen', false)"
    >
      <div class="space-y-3 py-2 text-xs">
        <div class="text-fg">
          {{
            cascadeAction === 'disable'
              ? t('resources.cascade.disableMessage', {
                  count: cascadeDependents.length,
                  names: cascadeDependents.join(', '),
                })
              : t('resources.cascade.deleteMessage', {
                  count: cascadeDependents.length,
                  names: cascadeDependents.join(', '),
                })
          }}
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end space-x-2">
          <MButton variant="tonal" @click="emit('update:cascadeOpen', false)">
            {{ t('resources.actions.cancel') }}
          </MButton>
          <MButton variant="tonal" @click="emit('cascadeConfirm')">
            {{ t('resources.cascade.onlyThis') }}
          </MButton>
          <MButton variant="danger" @click="emit('cascadeConfirm')">
            {{
              cascadeAction === 'disable'
                ? t('resources.cascade.cascadeDisable')
                : t('resources.cascade.cascadeDelete')
            }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Update Confirm Dialog -->
    <MDialog
      :open="updateOpen"
      :title="t('resources.update.confirmTitle')"
      class="max-w-md"
      @update:open="emit('update:updateOpen', $event)"
      @cancel="emit('update:updateOpen', false)"
    >
      <div v-if="updateTarget" class="space-y-3 py-2 text-xs">
        <div class="text-fg">
          {{
            t('resources.update.confirmMessage', {
              name: updateTarget.title || updateTarget.fileName,
              current: updateTarget.version || 'unknown',
              latest: updateTarget.latestFile?.name || '',
            })
          }}
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end space-x-2">
          <MButton variant="tonal" @click="emit('update:updateOpen', false)">
            {{ t('resources.actions.cancel') }}
          </MButton>
          <MButton variant="primary" @click="updateTarget && emit('singleUpdate', updateTarget)">
            {{ t('resources.actions.update') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Batch Update Risk Dialog -->
    <MDialog
      :open="batchUpdateRiskOpen"
      :title="t('resources.update.warningTitle')"
      class="max-w-md"
      @update:open="emit('update:batchUpdateRiskOpen', $event)"
      @cancel="emit('update:batchUpdateRiskOpen', false)"
    >
      <div class="space-y-3 py-2 text-xs text-fg">
        {{ t('resources.update.warningMessage', { count: pendingBatchUpdates.length }) }}
      </div>
      <template #footer>
        <div class="flex justify-end space-x-2">
          <MButton variant="tonal" @click="emit('update:batchUpdateRiskOpen', false)">
            {{ t('resources.actions.cancel') }}
          </MButton>
          <MButton variant="primary" @click="emit('batchUpdate', pendingBatchUpdates)">
            {{ t('resources.update.understoodRisk') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

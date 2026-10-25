<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { CompFile } from '../../../ipc/client'
import { MDialog, MButton, MSelect, MBadge, MCheckbox, MInput, MSkeleton } from '../../../ui'

export interface ResolvedDep {
  project: { id: string; title: string; slug: string }
  file: CompFile
  required?: boolean
  installed: boolean
}

defineProps<{
  instancesCount: number
  instanceOptions: Array<{ label: string; value: string }>
  compType?: string
  resolvingDependencies: boolean
  dependencies: ResolvedDep[]
  downloading: boolean
  installingModpack: boolean
}>()

const downloadDialogOpen = defineModel<boolean>('downloadDialogOpen', { required: true })
const modpackInstallDialogOpen = defineModel<boolean>('modpackInstallDialogOpen', {
  required: true,
})
const resolveFailedDialogOpen = defineModel<boolean>('resolveFailedDialogOpen', { required: true })
const cannotInstallDialogOpen = defineModel<boolean>('cannotInstallDialogOpen', { required: true })
const depConfirmDialogOpen = defineModel<boolean>('depConfirmDialogOpen', { required: true })

const selectedTargetInstance = defineModel<string>('selectedTargetInstance', { required: true })
const selectedDepFiles = defineModel<Record<string, boolean>>('selectedDepFiles', {
  required: true,
})
const modpackInstanceName = defineModel<string>('modpackInstanceName', { required: true })

const emit = defineEmits<{
  (e: 'resolve-dependencies'): void
  (e: 'start-download'): void
  (e: 'start-modpack-install'): void
  (e: 'do-download-files', withDeps: boolean): void
}>()

const { t } = useI18n()
</script>

<template>
  <!-- Download Instance Selection Dialog -->
  <MDialog
    :open="downloadDialogOpen"
    :title="t('comp.downloadDialog.title')"
    @update:open="downloadDialogOpen = $event"
    @cancel="downloadDialogOpen = false"
  >
    <div class="space-y-4 py-2">
      <div v-if="instancesCount === 0" class="text-sm text-danger">
        {{ t('comp.downloadDialog.noInstances') }}
      </div>
      <div v-else class="space-y-3">
        <label class="text-xs font-semibold text-fg-muted block">
          {{ t('comp.downloadDialog.selectInstance') }}
        </label>
        <MSelect
          v-model="selectedTargetInstance"
          :options="instanceOptions"
          @update:model-value="emit('resolve-dependencies')"
        />
        <div class="text-xs text-fg-muted">
          {{
            t('comp.downloadDialog.targetSubfolder', {
              folder: compType || 'mods',
            })
          }}
        </div>

        <!-- Dependencies Checklist -->
        <div
          v-if="compType === 'mod' && (resolvingDependencies || dependencies.length > 0)"
          class="pt-2 border-t border-line space-y-2"
        >
          <div class="text-xs font-semibold text-fg">
            {{ t('comp.downloadDialog.dependenciesTitle') }}
          </div>
          <div class="text-[11px] text-fg-muted">
            {{ t('comp.downloadDialog.dependenciesDesc') }}
          </div>

          <div v-if="resolvingDependencies" class="space-y-1.5 py-1">
            <MSkeleton class="h-6 rounded" />
            <MSkeleton class="h-6 rounded" />
          </div>

          <div v-else class="space-y-1.5 max-h-40 overflow-y-auto">
            <div
              v-for="dep in dependencies"
              :key="dep.file.id"
              class="flex items-center justify-between p-2 rounded-lg bg-sunken/30 text-xs"
            >
              <div class="flex items-center space-x-2 min-w-0">
                <MCheckbox v-if="!dep.installed" v-model="selectedDepFiles[dep.file.id]" />
                <span v-else class="i-lucide-check text-success shrink-0 w-4 h-4 ml-0.5" />
                <span class="font-medium text-fg truncate">
                  {{ dep.project.title }}
                </span>
                <span class="text-fg-muted font-mono text-[11px]">v{{ dep.file.name }}</span>
              </div>

              <div class="shrink-0">
                <MBadge :variant="dep.installed ? 'success' : 'neutral'" size="sm">
                  {{
                    dep.installed
                      ? t('comp.downloadDialog.depInstalled')
                      : t('comp.downloadDialog.depRequired')
                  }}
                </MBadge>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex items-center justify-end space-x-3">
        <MButton variant="text" @click="downloadDialogOpen = false">
          {{ t('comp.downloadDialog.cancel') }}
        </MButton>
        <MButton
          variant="primary"
          :loading="downloading"
          :disabled="instancesCount === 0 || !selectedTargetInstance"
          @click="emit('start-download')"
        >
          {{ t('comp.downloadDialog.confirm') }}
        </MButton>
      </div>
    </template>
  </MDialog>

  <!-- Modpack Install Dialog -->
  <MDialog
    :open="modpackInstallDialogOpen"
    :title="t('comp.modpackInstallDialog.title')"
    @update:open="modpackInstallDialogOpen = $event"
    @cancel="modpackInstallDialogOpen = false"
  >
    <div class="space-y-4 py-2">
      <div class="space-y-2">
        <label class="text-xs font-semibold text-fg-muted block">
          {{ t('comp.modpackInstallDialog.instanceName') }}
        </label>
        <MInput
          v-model="modpackInstanceName"
          :placeholder="t('comp.modpackInstallDialog.instanceNamePlaceholder')"
        />
      </div>
    </div>

    <template #footer>
      <div class="flex items-center justify-end space-x-3">
        <MButton variant="text" @click="modpackInstallDialogOpen = false">
          {{ t('comp.downloadDialog.cancel') }}
        </MButton>
        <MButton
          variant="primary"
          :loading="installingModpack"
          :disabled="!modpackInstanceName.trim()"
          @click="emit('start-modpack-install')"
        >
          {{ t('comp.modpackInstallDialog.install') }}
        </MButton>
      </div>
    </template>
  </MDialog>

  <!-- Resolve Failed Dialog -->
  <MDialog
    :open="resolveFailedDialogOpen"
    :title="t('comp.dependency.resolveFailedTitle')"
    @update:open="resolveFailedDialogOpen = $event"
    @cancel="resolveFailedDialogOpen = false"
  >
    <div class="py-2 text-xs text-fg">
      {{ t('comp.dependency.resolveFailedTitle') }}
    </div>
    <template #footer>
      <div class="flex items-center justify-end space-x-3">
        <MButton variant="text" @click="resolveFailedDialogOpen = false">
          {{ t('comp.downloadDialog.cancel') }}
        </MButton>
        <MButton variant="primary" @click="emit('do-download-files', false)">
          {{ t('comp.dependency.continueDownload') }}
        </MButton>
      </div>
    </template>
  </MDialog>

  <!-- Cannot Install Required Dependencies Dialog -->
  <MDialog
    :open="cannotInstallDialogOpen"
    :title="t('comp.dependency.cannotInstallTitle')"
    @update:open="cannotInstallDialogOpen = $event"
    @cancel="cannotInstallDialogOpen = false"
  >
    <div class="py-2 text-xs text-fg">
      {{ t('comp.dependency.cannotInstallTitle') }}
    </div>
    <template #footer>
      <div class="flex items-center justify-end space-x-3">
        <MButton variant="text" @click="cannotInstallDialogOpen = false">
          {{ t('comp.downloadDialog.cancel') }}
        </MButton>
        <MButton variant="primary" @click="emit('do-download-files', false)">
          {{ t('comp.dependency.continueDownload') }}
        </MButton>
      </div>
    </template>
  </MDialog>

  <!-- Confirm Dependencies Dialog -->
  <MDialog
    :open="depConfirmDialogOpen"
    :title="t('comp.dependency.confirmTitle')"
    @update:open="depConfirmDialogOpen = $event"
    @cancel="depConfirmDialogOpen = false"
  >
    <div class="py-2 text-xs text-fg">
      {{ t('comp.dependency.confirmTitle') }}
    </div>
    <template #footer>
      <div class="flex items-center justify-end space-x-2">
        <MButton variant="text" @click="depConfirmDialogOpen = false">
          {{ t('comp.downloadDialog.cancel') }}
        </MButton>
        <MButton variant="tonal" @click="emit('do-download-files', false)">
          {{ t('comp.dependency.installModOnly') }}
        </MButton>
        <MButton variant="primary" @click="emit('do-download-files', true)">
          {{ t('comp.dependency.installWithDeps') }}
        </MButton>
      </div>
    </template>
  </MDialog>
</template>

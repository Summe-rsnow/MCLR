<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient, type ModpackInspectResult } from '../../ipc/client'
import { MDialog, MButton, MInput, MPathPicker, MSkeleton, MBadge, useToast } from '../../ui'

defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'imported', instanceName: string): void
}>()

const { t } = useI18n()
const toast = useToast()

const archivePath = ref('')
const inspecting = ref(false)
const inspectResult = ref<ModpackInspectResult | null>(null)
const targetInstanceName = ref('')
const importing = ref(false)
const inspectError = ref('')

function onArchivePathChange(val: string) {
  archivePath.value = val
  if (val.trim()) {
    onInspect()
  }
}
async function onInspect(): Promise<void> {
  if (!archivePath.value.trim()) return
  inspecting.value = true
  inspectError.value = ''
  inspectResult.value = null
  try {
    const res = await ipcClient.modpackInspect(archivePath.value.trim())
    inspectResult.value = res
    if (res.name) {
      targetInstanceName.value = res.name
    } else {
      targetInstanceName.value = 'Modpack-' + Math.floor(Date.now() / 1000)
    }
  } catch (err: unknown) {
    inspectError.value = err instanceof Error ? err.message : String(err)
    toast.show({
      message: t('comp.modpackImport.inspectFailed') + ': ' + inspectError.value,
      type: 'danger',
    })
  } finally {
    inspecting.value = false
  }
}

async function onImport(): Promise<void> {
  if (!archivePath.value.trim() || !targetInstanceName.value.trim()) return
  importing.value = true
  try {
    await ipcClient.modpackImportFile(archivePath.value.trim(), targetInstanceName.value.trim())
    toast.show({
      message: t('comp.modpackImport.success'),
      type: 'success',
    })
    emit('imported', targetInstanceName.value.trim())
    emit('update:open', false)
    archivePath.value = ''
    inspectResult.value = null
    targetInstanceName.value = ''
  } catch (err: unknown) {
    toast.show({
      message:
        t('comp.modpackImport.failed') + ': ' + (err instanceof Error ? err.message : String(err)),
      type: 'danger',
    })
  } finally {
    importing.value = false
  }
}

function onClose(): void {
  emit('update:open', false)
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('comp.modpackImport.dialogTitle')"
    @update:open="emit('update:open', $event)"
    @cancel="onClose"
  >
    <div class="space-y-4 py-2">
      <!-- File Selector -->
      <div class="space-y-1.5">
        <label class="text-xs font-semibold text-fg-muted block">
          {{ t('comp.modpackImport.selectFile') }}
        </label>
        <MPathPicker
          v-model="archivePath"
          type="file"
          placeholder="C:\path\to\modpack.zip"
          :button-text="t('comp.modpackImport.browse')"
          :filters="[
            {
              name: 'Minecraft Modpack (*.mrpack, *.zip)',
              extensions: ['mrpack', 'zip'],
            },
          ]"
          @update:model-value="onArchivePathChange"
        />
      </div>

      <!-- Inspect Loading -->
      <div v-if="inspecting" class="py-3 space-y-2">
        <div class="text-xs text-fg-muted flex items-center space-x-2">
          <span class="i-lucide-loader-circle animate-spin w-4 h-4 text-primary" />
          <span>{{ t('comp.modpackImport.inspecting') }}</span>
        </div>
        <MSkeleton class="h-16 rounded-xl" />
      </div>

      <!-- Inspect Result -->
      <div
        v-else-if="inspectResult"
        class="p-3 rounded-xl bg-sunken/30 border border-line space-y-2 text-xs"
      >
        <div class="flex items-center justify-between">
          <span class="font-semibold text-fg text-sm">{{ inspectResult.name }}</span>
          <MBadge variant="primary">{{ inspectResult.format }}</MBadge>
        </div>

        <div class="grid grid-cols-2 gap-2 text-fg-muted">
          <div>{{ t('comp.modpackImport.mcVersion', { version: inspectResult.mcVersion }) }}</div>
          <div>
            {{
              t('comp.modpackImport.loader', {
                loader: inspectResult.loader || 'Vanilla',
                loaderVersion: inspectResult.loaderVersion || '',
              })
            }}
          </div>
          <div>{{ t('comp.modpackImport.fileCount', { count: inspectResult.fileCount }) }}</div>
          <div v-if="inspectResult.author">
            {{ t('comp.modpackImport.author', { author: inspectResult.author }) }}
          </div>
        </div>
      </div>

      <!-- Target Instance Name -->
      <div v-if="inspectResult" class="space-y-1.5 pt-2 border-t border-line">
        <label class="text-xs font-semibold text-fg-muted block">
          {{ t('comp.modpackImport.targetName') }}
        </label>
        <MInput
          v-model="targetInstanceName"
          :placeholder="t('comp.modpackImport.targetNamePlaceholder')"
        />
      </div>
    </div>

    <template #footer>
      <div class="flex items-center justify-end space-x-3">
        <MButton variant="text" @click="onClose">
          {{ t('comp.downloadDialog.cancel') }}
        </MButton>
        <MButton
          variant="primary"
          :loading="importing"
          :disabled="!archivePath || !inspectResult || !targetInstanceName.trim()"
          @click="onImport"
        >
          {{ t('comp.modpackImport.import') }}
        </MButton>
      </div>
    </template>
  </MDialog>
</template>

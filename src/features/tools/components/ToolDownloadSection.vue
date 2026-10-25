<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient } from '../../../ipc/client'
import { useToast } from '../../../ui/useToast'
import { MCard, MButton, MInput, MPathPicker, MHelpTooltip } from '../../../ui'

const { t } = useI18n()
const toast = useToast()

const downloadUrl = ref('')
const downloadDestPath = ref('')
const downloadLoading = ref(false)

const defaultDownloadFilename = computed(() => {
  return downloadUrl.value.split('/').pop()?.split('?')[0] || 'download.bin'
})

async function startCustomDownload() {
  const url = downloadUrl.value.trim()
  const dest = downloadDestPath.value.trim()
  if (!url) {
    toast.warning(t('tools.download.emptyUrl'))
    return
  }
  if (!dest) {
    toast.warning(t('tools.download.emptyDest'))
    return
  }
  downloadLoading.value = true
  try {
    await ipcClient.toolsCustomDownload(url, dest)
    toast.success(t('tools.download.started'))
    downloadUrl.value = ''
    downloadDestPath.value = ''
  } catch (err: unknown) {
    toast.danger(err instanceof Error ? err.message : String(err))
  } finally {
    downloadLoading.value = false
  }
}
</script>

<template>
  <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 2 }">
    <div>
      <div class="flex items-center gap-3">
        <div
          class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
        >
          <span class="i-lucide-cloud-download w-5 h-5 inline-block" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <h2 class="text-base font-semibold text-fg">{{ t('tools.download.title') }}</h2>
            <MHelpTooltip :text="t('tools.download.desc')" />
          </div>
          <p class="text-xs text-fg-muted mt-0.5 truncate">
            {{ t('tools.download.desc') }}
          </p>
        </div>
      </div>
      <div class="mt-4 space-y-3">
        <div>
          <label class="text-xs text-fg-muted block mb-1 font-medium">{{
            t('tools.download.urlLabel')
          }}</label>
          <MInput
            v-model="downloadUrl"
            :placeholder="t('tools.download.urlPlaceholder')"
            class="w-full"
          />
        </div>

        <div>
          <label class="text-xs text-fg-muted block mb-1 font-medium">{{
            t('tools.download.destLabel')
          }}</label>
          <MPathPicker
            v-model="downloadDestPath"
            type="saveFile"
            :placeholder="t('tools.download.destPlaceholder')"
            :button-text="t('tools.download.browse')"
            :default-path="downloadDestPath || defaultDownloadFilename"
          />
        </div>
      </div>
    </div>
    <div class="pt-2 flex justify-end">
      <MButton
        variant="primary"
        icon="i-lucide-play"
        :loading="downloadLoading"
        data-testid="custom-download-btn"
        @click="startCustomDownload"
      >
        {{ t('tools.download.start') }}
      </MButton>
    </div>
  </MCard>
</template>

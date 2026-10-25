<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import { ipcClient, type SkinFetchResult } from '../../../ipc/client'
import { useToast } from '../../../ui/useToast'
import { useErrorText } from '../../../composables/useErrorText'
import { MCard, MButton, MInput, MSkeleton, MHelpTooltip, MImage } from '../../../ui'

const { t } = useI18n()
const toast = useToast()
const { formatError } = useErrorText()

const skinUsername = ref('')
const skinLoading = ref(false)
const skinResult = ref<SkinFetchResult | null>(null)

async function fetchSkin() {
  const name = skinUsername.value.trim()
  if (!name) {
    toast.warning(t('tools.skin.usernamePlaceholder'))
    return
  }
  skinLoading.value = true
  try {
    skinResult.value = await ipcClient.toolsFetchSkin(name)
  } catch (err: unknown) {
    toast.danger(formatError(err))
  } finally {
    skinLoading.value = false
  }
}

async function downloadSkinPng() {
  if (!skinResult.value) return
  try {
    const defaultName = `${skinResult.value.username}_skin.png`
    const path = await saveFileDialog({
      defaultPath: defaultName,
      filters: [{ name: 'PNG Image', extensions: ['png'] }],
    })
    if (!path || typeof path !== 'string') return

    await ipcClient.toolsSaveSkin(path, skinResult.value.skinDataUrl)
    toast.success(t('tools.skin.downloadSuccess'))
  } catch (err: unknown) {
    toast.danger(formatError(err))
  }
}
</script>

<template>
  <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 0 }">
    <div>
      <div class="flex items-center gap-3">
        <div
          class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
        >
          <span class="i-lucide-user-check w-5 h-5 inline-block" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <h2 class="text-base font-semibold text-fg">{{ t('tools.skin.title') }}</h2>
            <MHelpTooltip :text="t('tools.skin.desc')" />
          </div>
          <p class="text-xs text-fg-muted mt-0.5 truncate">{{ t('tools.skin.desc') }}</p>
        </div>
      </div>

      <div class="flex items-center gap-2 mt-4">
        <MInput
          v-model="skinUsername"
          :placeholder="t('tools.skin.usernamePlaceholder')"
          class="flex-1"
          @keydown.enter="fetchSkin"
        />
        <MButton
          variant="primary"
          :loading="skinLoading"
          data-testid="fetch-skin-btn"
          @click="fetchSkin"
        >
          {{ t('tools.skin.fetch') }}
        </MButton>
      </div>

      <!-- Skin Loading Skeleton -->
      <div v-if="skinLoading" class="mt-4 p-4 rounded-xl m-well space-y-3">
        <div class="flex items-center gap-4">
          <MSkeleton class="w-16 h-16 rounded-lg shrink-0" />
          <div class="flex-1 min-w-0 space-y-2">
            <MSkeleton class="h-4 w-28" />
            <MSkeleton class="h-3 w-44" />
            <MSkeleton class="h-3 w-20" />
          </div>
        </div>
        <MSkeleton class="aspect-square w-48 mx-auto rounded-xl" />
      </div>

      <!-- Result Display -->
      <div v-else-if="skinResult" class="mt-4 p-4 rounded-xl m-well space-y-3">
        <div class="flex items-center gap-4">
          <div class="flex flex-col items-center">
            <span class="text-xs text-fg-muted mb-1">{{ t('tools.skin.headPreview') }}</span>
            <MImage
              :src="skinResult.headDataUrl"
              :alt="skinResult.username"
              pixelated
              fallback-icon="i-lucide-user"
              class="w-16 h-16 rounded-lg border border-line bg-raised"
            />
          </div>

          <div class="flex-1 min-w-0 space-y-1 text-xs select-text">
            <div class="font-bold text-sm text-fg truncate">{{ skinResult.username }}</div>
            <div class="text-fg-muted truncate">
              <span class="font-medium">{{ t('tools.skin.uuid') }}:</span>
              <span class="font-mono ml-1">{{ skinResult.uuid }}</span>
            </div>
            <div class="text-fg-muted">
              <span class="font-medium">{{ t('tools.skin.model') }}:</span>
              <span class="ml-1 px-1.5 py-0.5 rounded bg-primary/10 text-primary font-medium">
                {{ skinResult.slim ? t('tools.skin.modelSlim') : t('tools.skin.modelClassic') }}
              </span>
            </div>
          </div>
        </div>
        <div>
          <span class="text-xs text-fg-muted block mb-1">{{ t('tools.skin.skinPreview') }}</span>
          <div class="p-2 rounded-lg bg-raised border border-line flex justify-center">
            <MImage
              :src="skinResult.skinDataUrl"
              alt="Skin Texture"
              pixelated
              fit="contain"
              fallback-icon="i-lucide-shirt"
              class="max-h-24 w-auto"
            />
          </div>
        </div>

        <div class="pt-1 flex justify-end">
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-download"
            data-testid="download-skin-btn"
            @click="downloadSkinPng"
          >
            {{ t('tools.skin.downloadSkin') }}
          </MButton>
        </div>
      </div>
    </div>
  </MCard>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import { open as openFileDialog, save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import { ipcClient } from '../../../ipc/client'
import { useToast } from '../../../ui/useToast'
import { useErrorText } from '../../../composables/useErrorText'
import { MCard, MButton, MSegmented, MSkeleton, MImage, MHelpTooltip } from '../../../ui'

const { t } = useI18n()
const toast = useToast()
const { formatError } = useErrorText()

const AVATAR_SIZES = [64, 128, 256, 512] as const
const sizeOptions = AVATAR_SIZES.map((s) => ({ label: `${s}px`, value: String(s) }))

const sourcePath = ref('')
const avatarSize = ref<number>(128)
const sizeModel = computed<string>({
  get: () => String(avatarSize.value),
  set: (v) => {
    void onSizeChange(Number(v))
  },
})
const avatarDataUrl = ref('')
const generating = ref(false)

const sourceName = computed(() => {
  if (!sourcePath.value) return ''
  const parts = sourcePath.value.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || sourcePath.value
})

const sourcePreview = computed(() => {
  if (!sourcePath.value) return ''
  try {
    return convertFileSrc(sourcePath.value)
  } catch {
    return ''
  }
})

async function generate(path: string, size: number) {
  if (!path) return
  generating.value = true
  try {
    avatarDataUrl.value = await ipcClient.toolsGenerateSkinAvatar(path, size)
  } catch (err: unknown) {
    avatarDataUrl.value = ''
    toast.danger(formatError(err))
  } finally {
    generating.value = false
  }
}

async function onPickSkin() {
  try {
    const selected = await openFileDialog({
      filters: [{ name: t('tools.skinAvatar.fileFilter'), extensions: ['png'] }],
      multiple: false,
    })
    if (!selected || typeof selected !== 'string') return
    sourcePath.value = selected
    await generate(selected, avatarSize.value)
    if (avatarDataUrl.value) {
      toast.success(t('tools.skinAvatar.generated'))
    }
  } catch (err: unknown) {
    toast.danger(formatError(err))
  }
}

async function onSizeChange(size: number) {
  avatarSize.value = size
  if (sourcePath.value) await generate(sourcePath.value, size)
}

async function onSaveAvatar() {
  if (!avatarDataUrl.value) {
    toast.warning(t('tools.skinAvatar.selectFirst'))
    return
  }
  try {
    const base = sourceName.value.replace(/\.png$/i, '') || 'skin'
    const path = await saveFileDialog({
      defaultPath: `${base}_head_${avatarSize.value}.png`,
      filters: [{ name: t('tools.skinAvatar.fileFilter'), extensions: ['png'] }],
    })
    if (!path || typeof path !== 'string') return
    await ipcClient.toolsSaveSkin(path, avatarDataUrl.value)
    toast.success(t('tools.skinAvatar.saved'))
  } catch (err: unknown) {
    toast.danger(formatError(err))
  }
}
</script>

<template>
  <MCard class="flex flex-col gap-4">
    <div class="flex items-start justify-between gap-3">
      <div class="flex items-center gap-2.5 min-w-0">
        <div
          class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
        >
          <span class="i-lucide-scan-face w-4 h-4" />
        </div>
        <div class="min-w-0">
          <h3 class="text-sm font-semibold text-fg flex items-center gap-1.5">
            {{ t('tools.skinAvatar.title') }}
            <MHelpTooltip :text="t('tools.skinAvatar.description')" size="xs" />
          </h3>
          <p class="text-xs text-fg-muted truncate mt-0.5">{{ t('tools.skinAvatar.subtitle') }}</p>
        </div>
      </div>
    </div>

    <!-- Source picker -->
    <div class="flex flex-wrap items-center gap-2">
      <MButton variant="primary" size="sm" icon="i-lucide-image-plus" @click="onPickSkin">
        {{ t('tools.skinAvatar.selectSkin') }}
      </MButton>
      <MButton
        v-if="avatarDataUrl"
        variant="tonal"
        size="sm"
        icon="i-lucide-save"
        @click="onSaveAvatar"
      >
        {{ t('tools.skinAvatar.save') }}
      </MButton>
      <span v-if="sourceName" class="text-xs text-fg-muted font-mono truncate max-w-[220px]">
        {{ sourceName }}
      </span>
    </div>

    <!-- Avatar size -->
    <div class="flex flex-wrap items-center gap-3">
      <span class="text-xs font-medium text-fg-muted">{{ t('tools.skinAvatar.size') }}</span>
      <MSegmented v-model="sizeModel" :options="sizeOptions" size="sm" />
    </div>

    <!-- Preview -->
    <div class="flex flex-wrap items-center gap-4">
      <div class="flex flex-col items-center gap-1.5">
        <span class="text-[11px] text-fg-muted">{{ t('tools.skinAvatar.sourceLabel') }}</span>
        <div
          class="w-20 h-20 rounded-xl border border-line bg-sunken/40 flex items-center justify-center overflow-hidden"
        >
          <MImage
            :src="sourcePreview"
            alt="skin"
            pixelated
            fit="contain"
            class="w-full h-full"
            fallback-icon="i-lucide-shirt"
          />
        </div>
      </div>

      <span class="i-lucide-arrow-right w-4 h-4 text-fg-subtle shrink-0" />

      <div class="flex flex-col items-center gap-1.5">
        <span class="text-[11px] text-fg-muted">{{ t('tools.skinAvatar.avatarLabel') }}</span>
        <div
          class="avatar-checker w-20 h-20 rounded-xl border border-line flex items-center justify-center overflow-hidden"
        >
          <MSkeleton v-if="generating" class="w-full h-full" :rounded="false" />
          <MImage
            v-else
            :src="avatarDataUrl"
            alt="avatar"
            pixelated
            fit="contain"
            class="w-full h-full"
            fallback-icon="i-lucide-scan-face"
          />
        </div>
      </div>
    </div>
  </MCard>
</template>

<style scoped>
.avatar-checker {
  background-image:
    linear-gradient(45deg, rgb(var(--c-line) / 0.5) 25%, transparent 25%),
    linear-gradient(-45deg, rgb(var(--c-line) / 0.5) 25%, transparent 25%),
    linear-gradient(45deg, transparent 75%, rgb(var(--c-line) / 0.5) 75%),
    linear-gradient(-45deg, transparent 75%, rgb(var(--c-line) / 0.5) 75%);
  background-size: 12px 12px;
  background-position:
    0 0,
    0 6px,
    6px -6px,
    -6px 0;
  background-color: rgb(var(--c-raised));
}
</style>

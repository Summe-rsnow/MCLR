<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import {
  ipcClient,
  type SkinFetchResult,
  type CacheSizeInfo,
} from '../../ipc/client'
import { useToast } from '../../ui/useToast'
import { useInstancesStore } from '../instances/store'
import {
  MCard,
  MButton,
  MInput,
  MPathPicker,
  MSelect,
  MCheckbox,
  MPageHeader,
  MSkeletonList,
  MHelpTooltip,
} from '../../ui'
const { t } = useI18n()
const toast = useToast()
const instancesStore = useInstancesStore()

const initialized = ref(false)

// --- Tool 1: Skin Fetcher ---
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
    toast.danger(err instanceof Error ? err.message : String(err))
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

    // In desktop app or mock, download via fetch
    const res = await fetch(skinResult.value.skinDataUrl)
    const blob = await res.blob()
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = defaultName
    a.click()
    URL.revokeObjectURL(url)

    toast.success(t('tools.skin.downloadSuccess'))
  } catch (err: unknown) {
    toast.danger(err instanceof Error ? err.message : String(err))
  }
}

// --- Tool 2: Cache Cleaner ---
const cacheInfo = ref<CacheSizeInfo>({
  cacheBytes: 0,
  logsBytes: 0,
  skinCacheBytes: 0,
  totalBytes: 0,
})
const cleanCacheChecked = ref(true)
const cleanLogsChecked = ref(false)
const cleanSkinsChecked = ref(false)
const cacheCleaning = ref(false)
const cacheCleanedNotice = ref(false)
async function loadCacheInfo() {
  try {
    cacheInfo.value = await ipcClient.toolsCacheInfo()
  } catch (err: unknown) {
    console.error('Failed to load cache info:', err)
  }
}

async function cleanCache() {
  if (!cleanCacheChecked.value && !cleanLogsChecked.value && !cleanSkinsChecked.value) {
    toast.warning(t('tools.cache.noneSelected'))
    return
  }
  cacheCleaning.value = true
  try {
    cacheInfo.value = await ipcClient.toolsCacheClear(
      cleanCacheChecked.value,
      cleanLogsChecked.value,
      cleanSkinsChecked.value,
    )
    toast.success(t('tools.cache.cleaned'))
    cacheCleanedNotice.value = true
    setTimeout(() => {
      cacheCleanedNotice.value = false
    }, 5000)
  } catch (err: unknown) {
    toast.danger(err instanceof Error ? err.message : String(err))
  } finally {
    cacheCleaning.value = false
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

// --- Tool 3: Custom Downloader ---
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

// --- Tool 4: Desktop Shortcut ---
const shortcutInstanceId = ref('')
const shortcutName = ref('')
const shortcutLoading = ref(false)

const instanceOptions = computed(() => {
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.id,
  }))
})

async function createShortcut() {
  if (!shortcutInstanceId.value) {
    toast.warning(t('tools.shortcut.noInstance'))
    return
  }
  shortcutLoading.value = true
  try {
    await ipcClient.toolsCreateShortcut(
      shortcutInstanceId.value,
      shortcutName.value.trim() || null,
    )
    toast.success(t('tools.shortcut.created'))
    shortcutName.value = ''
  } catch (err: unknown) {
    toast.danger(err instanceof Error ? err.message : String(err))
  } finally {
    shortcutLoading.value = false
  }
}

onMounted(async () => {
  try {
    await Promise.all([
      loadCacheInfo(),
      instancesStore.fetchInstances(),
    ])
    if (instancesStore.instances.length > 0 && !shortcutInstanceId.value) {
      shortcutInstanceId.value = instancesStore.instances[0].id
    }
  } finally {
    initialized.value = true
  }
})
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('tools.title')"
      :description="t('tools.subtitle')"
      icon="i-lucide-wrench"
    />

    <div v-if="!initialized" class="space-y-6">
      <MSkeletonList :rows="4" variant="card" />
    </div>

    <template v-else>
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6 items-start">
        <!-- Left Column: Card 1 (Skin) & Card 3 (Downloader) -->
        <div class="flex flex-col gap-6">
          <!-- Card 1: Avatar / Skin Fetcher -->
          <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 0 }">
            <div>
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
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

              <!-- Result Display -->
              <div v-if="skinResult" class="mt-4 p-4 rounded-xl m-well space-y-3">
                <div class="flex items-center gap-4">
                  <div class="flex flex-col items-center">
                    <span class="text-xs text-fg-muted mb-1">{{ t('tools.skin.headPreview') }}</span>
                    <img
                      :src="skinResult.headDataUrl"
                      :alt="skinResult.username"
                      class="w-16 h-16 rounded-lg border border-line bg-raised [image-rendering:pixelated]"
                      loading="lazy"
                      decoding="async"
                    />
                  </div>

                  <div class="flex-1 min-w-0 space-y-1 text-xs">
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
                    <img
                      :src="skinResult.skinDataUrl"
                      alt="Skin Texture"
                      class="max-h-24 w-auto [image-rendering:pixelated]"
                      loading="lazy"
                      decoding="async"
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

          <!-- Card 3: Custom Downloader -->
          <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 2 }">
            <div>
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
                  <span class="i-lucide-cloud-download w-5 h-5 inline-block" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-1.5">
                    <h2 class="text-base font-semibold text-fg">{{ t('tools.download.title') }}</h2>
                    <MHelpTooltip :text="t('tools.download.desc')" />
                  </div>
                  <p class="text-xs text-fg-muted mt-0.5 truncate">{{ t('tools.download.desc') }}</p>
                </div>
              </div>
              <div class="mt-4 space-y-3">
                <div>
                  <label class="text-xs text-fg-muted block mb-1 font-medium">{{ t('tools.download.urlLabel') }}</label>
                  <MInput
                    v-model="downloadUrl"
                    :placeholder="t('tools.download.urlPlaceholder')"
                    class="w-full"
                  />
                </div>

                <div>
                  <label class="text-xs text-fg-muted block mb-1 font-medium">{{ t('tools.download.destLabel') }}</label>
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
        </div>

        <!-- Right Column: Card 2 (Cache) & Card 4 (Shortcut) -->
        <div class="flex flex-col gap-6">
          <!-- Card 2: Cache Cleaner -->
          <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 1 }">
            <div>
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
                  <span class="i-lucide-trash-2 w-5 h-5 inline-block" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-1.5">
                    <h2 class="text-base font-semibold text-fg">{{ t('tools.cache.title') }}</h2>
                    <MHelpTooltip :text="t('tools.cache.desc')" />
                  </div>
                  <p class="text-xs text-fg-muted mt-0.5 truncate">{{ t('tools.cache.desc') }}</p>
                </div>
              </div>
              <div class="mt-4 space-y-3">
                <div class="flex items-center justify-between p-3 rounded-xl m-well">
                  <MCheckbox v-model="cleanCacheChecked" :label="t('tools.cache.itemCache')" />
                  <span class="text-xs font-mono font-semibold text-fg tabular-nums">
                    {{ formatBytes(cacheInfo.cacheBytes) }}
                  </span>
                </div>

                <div class="flex items-center justify-between p-3 rounded-xl m-well">
                  <MCheckbox v-model="cleanLogsChecked" :label="t('tools.cache.itemLogs')" />
                  <span class="text-xs font-mono font-semibold text-fg tabular-nums">
                    {{ formatBytes(cacheInfo.logsBytes) }}
                  </span>
                </div>

                <div class="flex items-center justify-between p-3 rounded-xl m-well">
                  <MCheckbox v-model="cleanSkinsChecked" :label="t('tools.cache.itemSkins')" />
                  <span class="text-xs font-mono font-semibold text-fg tabular-nums">
                    {{ formatBytes(cacheInfo.skinCacheBytes) }}
                  </span>
                </div>
              </div>
            </div>
            <div v-if="cacheCleanedNotice" class="text-xs text-primary font-medium">
              {{ t('tools.cache.cleaned') }}
            </div>
            <div class="pt-2 border-t border-line flex items-center justify-between">
              <div class="text-xs text-fg-muted">
                {{ t('tools.cache.totalSize') }}:
                <span class="font-mono font-bold text-sm text-fg ml-1">
                  {{ formatBytes(cacheInfo.totalBytes) }}
                </span>
              </div>

              <MButton
                variant="danger"
                icon="i-lucide-trash"
                :loading="cacheCleaning"
                data-testid="clean-cache-btn"
                @click="cleanCache"
              >
                {{ t('tools.cache.cleanNow') }}
              </MButton>
            </div>
          </MCard>

          <!-- Card 4: Desktop Shortcut -->
          <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 3 }">
            <div>
              <div class="flex items-center gap-3">
                <div class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
                  <span class="i-lucide-external-link w-5 h-5 inline-block" />
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center gap-1.5">
                    <h2 class="text-base font-semibold text-fg">{{ t('tools.shortcut.title') }}</h2>
                    <MHelpTooltip :text="t('tools.shortcut.desc')" />
                  </div>
                  <p class="text-xs text-fg-muted mt-0.5 truncate">{{ t('tools.shortcut.desc') }}</p>
                </div>
              </div>
              <div class="mt-4 space-y-3">
                <div>
                  <label class="text-xs text-fg-muted block mb-1 font-medium">{{ t('tools.shortcut.selectInstance') }}</label>
                  <MSelect
                    v-model="shortcutInstanceId"
                    :options="instanceOptions"
                    class="w-full"
                  />
                </div>

                <div>
                  <label class="text-xs text-fg-muted block mb-1 font-medium">{{ t('tools.shortcut.nameLabel') }}</label>
                  <MInput
                    v-model="shortcutName"
                    :placeholder="t('tools.shortcut.namePlaceholder')"
                    class="w-full"
                  />
                </div>
              </div>
            </div>
            <div class="pt-2 flex justify-end">
              <MButton
                variant="primary"
                icon="i-lucide-monitor"
                :loading="shortcutLoading"
                data-testid="create-shortcut-btn"
                @click="createShortcut"
              >
                {{ t('tools.shortcut.create') }}
              </MButton>
            </div>
          </MCard>
        </div>
      </div>
    </template>
  </div>
</template>

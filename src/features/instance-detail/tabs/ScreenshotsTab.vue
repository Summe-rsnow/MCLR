<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { convertFileSrc } from '@tauri-apps/api/core'
import type { ScreenshotMeta } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import {
  MCard,
  MButton,
  MIconButton,
  MEmpty,
  MSkeletonList,
  useToast,
  useDialog,
} from '../../../ui'

const props = defineProps<{
  instanceId: string
}>()

const { t } = useI18n()
const toast = useToast()
const { confirm } = useDialog()

const screenshots = ref<ScreenshotMeta[]>([])
const total = ref(0)
const loading = ref(false)
const loadingMore = ref(false)
const thumbs = ref<Record<string, string>>({})
const loaded = ref<Record<string, boolean>>({})

const sentinelRef = ref<HTMLElement | null>(null)
let observer: IntersectionObserver | null = null
const PAGE_SIZE = 20

async function fetchThumbs(items: ScreenshotMeta[]) {
  for (const item of items) {
    if (!thumbs.value[item.path]) {
      ipcClient
        .screenshotThumb(item.path)
        .then((thumbPath) => {
          thumbs.value[item.path] = convertFileSrc(thumbPath)
        })
        .catch(() => {
          // fallback to direct file if thumbnail generation fails
          thumbs.value[item.path] = convertFileSrc(item.path)
        })
    }
  }
}

async function fetchInitial() {
  loading.value = true
  try {
    const res = await ipcClient.screenshotsListPaged(props.instanceId, 0, PAGE_SIZE)
    screenshots.value = res.items
    total.value = res.total
    fetchThumbs(res.items)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loading.value = false
  }
}

async function fetchMore() {
  if (loadingMore.value || loading.value || screenshots.value.length >= total.value) return
  loadingMore.value = true
  try {
    const res = await ipcClient.screenshotsListPaged(
      props.instanceId,
      screenshots.value.length,
      PAGE_SIZE,
    )
    screenshots.value.push(...res.items)
    total.value = res.total
    fetchThumbs(res.items)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loadingMore.value = false
  }
}

function setupObserver() {
  if (typeof IntersectionObserver === 'undefined') return
  if (observer) observer.disconnect()

  observer = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting) {
        fetchMore()
      }
    },
    { rootMargin: '200px' },
  )

  if (sentinelRef.value) {
    observer.observe(sentinelRef.value)
  }
}

async function handleOpenFolder() {
  try {
    await ipcClient.instanceOpenPath(props.instanceId, 'screenshots')
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleOpen(item: ScreenshotMeta) {
  try {
    await ipcClient.screenshotOpen(item.path)
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleReveal(item: ScreenshotMeta) {
  try {
    await ipcClient.screenshotReveal(item.path)
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleCopy(item: ScreenshotMeta) {
  try {
    await ipcClient.screenshotCopy(item.path)
    toast.success(t('instanceDetail.screenshots.copySuccess', t('common.copied')))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function handleDelete(item: ScreenshotMeta) {
  const ok = await confirm({
    title: t('instanceDetail.screenshots.deleteConfirmTitle'),
    content: t('instanceDetail.screenshots.deleteConfirm', { name: item.fileName }),
  })
  if (!ok) return

  try {
    await ipcClient.screenshotDelete(item.path)
    screenshots.value = screenshots.value.filter((s) => s.path !== item.path)
    total.value = Math.max(0, total.value - 1)
    toast.success(t('instanceDetail.screenshots.deleteSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

onMounted(async () => {
  await fetchInitial()
  await nextTick()
  setupObserver()
})

onUnmounted(() => {
  if (observer) {
    observer.disconnect()
    observer = null
  }
})

watch(
  () => props.instanceId,
  () => {
    screenshots.value = []
    thumbs.value = {}
    loaded.value = {}
    fetchInitial()
  },
)
</script>

<template>
  <div class="max-w-4xl space-y-4">
    <!-- Top Bar -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <h2 class="text-base font-bold text-fg">
          {{ t('instanceDetail.tabs.screenshots') }}
        </h2>
        <span class="text-xs text-fg-muted font-mono">
          ({{ total }})
        </span>
      </div>
      <div class="flex items-center gap-2">
        <MButton variant="tonal" size="sm" @click="handleOpenFolder">
          <span class="w-4 h-4 i-lucide-folder-open mr-1" />
          {{ t('instanceDetail.screenshots.openFolder') }}
        </MButton>
        <MButton variant="tonal" size="sm" :loading="loading" @click="fetchInitial">
          <span class="w-4 h-4 i-lucide-refresh-cw mr-1" />
          {{ t('instanceDetail.screenshots.refresh') }}
        </MButton>
      </div>
    </div>

    <!-- Skeleton Loading Initial -->
    <div v-if="loading && screenshots.length === 0" class="py-4">
      <MSkeletonList :rows="6" variant="tile" />
    </div>

    <!-- Empty State -->
    <div v-else-if="screenshots.length === 0" class="py-16">
      <MEmpty
        :title="t('instanceDetail.screenshots.empty')"
        icon="i-lucide-image"
      />
    </div>

    <!-- Screenshots Grid -->
    <div v-else class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4">
      <MCard
        v-for="item in screenshots"
        :key="item.path"
        variant="well"
        hoverable
        padding="none"
        class="overflow-hidden group flex flex-col justify-between transition-all"
      >
        <!-- Thumbnail / Image -->
        <div
          class="aspect-video bg-sunken/50 relative overflow-hidden cursor-pointer flex items-center justify-center"
          :title="t('instanceDetail.screenshots.preview', t('common.open'))"
          @click="handleOpen(item)"
        >
          <img
            v-if="thumbs[item.path]"
            :src="thumbs[item.path]"
            :alt="item.fileName"
            class="w-full h-full object-cover transition-all duration-200"
            :class="loaded[item.path] ? 'opacity-100 scale-100' : 'opacity-0 scale-95'"
            loading="lazy"
            decoding="async"
            @load="loaded[item.path] = true"
          />
          <div
            v-else
            class="w-full h-full flex items-center justify-center bg-sunken/40"
          >
            <span class="w-8 h-8 i-lucide-image text-fg-muted/40" />
          </div>

          <div class="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center text-white">
            <span class="w-6 h-6 i-lucide-external-link" />
          </div>
        </div>

        <!-- Info & Actions Bar -->
        <div class="p-3 flex items-center justify-between gap-2 border-t border-line">
          <div class="min-w-0">
            <div class="text-xs font-semibold text-fg truncate" :title="item.fileName">
              {{ item.fileName }}
            </div>
            <div class="text-[11px] text-fg-muted mt-0.5">
              {{ formatBytes(item.size) }}
            </div>
          </div>

          <div class="flex items-center gap-1 shrink-0">
            <MIconButton
              icon="i-lucide-copy"
              size="sm"
              class="text-fg-muted hover:text-fg"
              :title="t('instanceDetail.screenshots.copy', t('common.copy'))"
              @click.stop="handleCopy(item)"
            />
            <MIconButton
              icon="i-lucide-folder-symlink"
              size="sm"
              class="text-fg-muted hover:text-fg"
              :title="t('instanceDetail.screenshots.reveal', t('common.openFolder'))"
              @click.stop="handleReveal(item)"
            />
            <MIconButton
              icon="i-lucide-trash-2"
              size="sm"
              class="text-danger hover:bg-danger/10"
              :title="t('instanceDetail.screenshots.delete')"
              @click.stop="handleDelete(item)"
            />
          </div>
        </div>
      </MCard>
    </div>

    <!-- Infinite scroll sentinel -->
    <div ref="sentinelRef" class="h-8 flex items-center justify-center">
      <span v-if="loadingMore" class="text-xs text-fg-muted animate-pulse">
        ...
      </span>
    </div>
  </div>
</template>

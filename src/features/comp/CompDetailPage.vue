<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { useCompStore } from './store'
import { useInstancesStore } from '../instances/store'
import type { CompPlatform, CompFile, CompType } from '../../ipc/client'
import {
  MButton,
  MIconButton,
  MCard,
  MBadge,
  MDialog,
  MSelect,
  MSkeleton,
  MEmpty,
  useToast,
} from '../../ui'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const toast = useToast()
const compStore = useCompStore()
const instancesStore = useInstancesStore()

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
})

const platform = computed(() => route.params.platform as CompPlatform)
const projectId = computed(() => route.params.id as string)

const selectedMcVersion = ref<string>('')
const selectedLoader = ref<string>('')

// Download Dialog state
const downloadDialogOpen = ref(false)
const selectedFile = ref<CompFile | null>(null)
const selectedTargetInstance = ref<string>('')
const downloading = ref(false)

const renderedDescription = computed(() => {
  const desc = compStore.currentProject?.description
  if (!desc) return ''
  const rawHtml = md.render(desc)
  return DOMPurify.sanitize(rawHtml)
})

const filteredFiles = computed(() => {
  let files = compStore.projectFiles
  if (selectedMcVersion.value) {
    files = files.filter((f) => f.mcVersions.includes(selectedMcVersion.value))
  }
  if (selectedLoader.value) {
    files = files.filter((f) => f.loaders.includes(selectedLoader.value))
  }
  return files
})

const mcVersionFilterOptions = computed(() => {
  const versions = new Set<string>()
  for (const f of compStore.projectFiles) {
    for (const v of f.mcVersions) {
      versions.add(v)
    }
  }
  return [
    { label: t('comp.search.allMcVersions'), value: '' },
    ...Array.from(versions).map((v) => ({ label: v, value: v })),
  ]
})

const loaderFilterOptions = computed(() => {
  const loaders = new Set<string>()
  for (const f of compStore.projectFiles) {
    for (const l of f.loaders) {
      loaders.add(l)
    }
  }
  return [
    { label: t('comp.search.allLoaders'), value: '' },
    ...Array.from(loaders).map((l) => ({ label: l, value: l })),
  ]
})

const instanceOptions = computed(() => {
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.name,
  }))
})

function formatFileSize(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    return (bytes / (1024 * 1024)).toFixed(2) + ' MB'
  }
  if (bytes >= 1024) {
    return (bytes / 1024).toFixed(1) + ' KB'
  }
  return bytes + ' B'
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr)
    return d.toLocaleDateString()
  } catch {
    return dateStr
  }
}

function getChannelVariant(channel: string): 'primary' | 'warning' | 'danger' {
  if (channel === 'release') return 'primary'
  if (channel === 'beta') return 'warning'
  return 'danger'
}

function openDownloadDialog(file: CompFile): void {
  selectedFile.value = file
  if (instancesStore.instances.length > 0) {
    const match = instancesStore.instances.find((i) =>
      file.mcVersions.includes(i.mcVersion),
    )
    selectedTargetInstance.value = match ? match.name : instancesStore.instances[0].name
  } else {
    selectedTargetInstance.value = ''
  }
  downloadDialogOpen.value = true
}

async function startDownload(): Promise<void> {
  if (!selectedFile.value || !selectedTargetInstance.value) return
  downloading.value = true
  try {
    const targetKind: CompType = compStore.currentProject?.compType || 'mod'
    await compStore.downloadFile(
      selectedFile.value,
      selectedTargetInstance.value,
      targetKind,
    )
    toast.show({
      message: t('comp.downloadDialog.success'),
      type: 'success',
    })
    downloadDialogOpen.value = false
  } catch (err: unknown) {
    toast.show({
      message: `${t('comp.downloadDialog.failed')}: ${err instanceof Error ? err.message : String(err)}`,
      type: 'danger',
    })
  } finally {
    downloading.value = false
  }
}

function openExternalUrl(url?: string | null): void {
  if (url) {
    window.open(url, '_blank')
  }
}

onMounted(async () => {
  if (instancesStore.instances.length === 0) {
    await instancesStore.fetchInstances()
  }
  await compStore.loadFavorites()
  if (platform.value && projectId.value) {
    await compStore.openProject(platform.value, projectId.value)
  }
})
</script>

<template>
  <div class="h-full flex flex-col p-6 space-y-6 overflow-y-auto">
    <!-- Back Button -->
    <div class="flex items-center space-x-3">
      <MIconButton
        icon="i-lucide-arrow-left"
        size="sm"
        @click="router.back()"
      />
      <span class="text-sm font-semibold text-text-muted">
        {{ t('comp.nav') }}
      </span>
    </div>

    <!-- Loading State -->
    <div v-if="compStore.loading" class="space-y-4">
      <MSkeleton class="h-28 rounded-xl" />
      <MSkeleton class="h-64 rounded-xl" />
    </div>

    <!-- Project Content -->
    <div v-else-if="compStore.currentProject" class="space-y-6">
      <!-- Header Card -->
      <MCard class="p-6">
        <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div class="flex items-start space-x-4">
            <img
              v-if="compStore.currentProject.iconUrl"
              :src="compStore.currentProject.iconUrl"
              class="w-20 h-20 rounded-xl object-cover bg-surface-variant flex-shrink-0"
              alt=""
            />
            <div
              v-else
              class="w-20 h-20 rounded-xl bg-surface-variant flex items-center justify-center flex-shrink-0"
            >
              <i class="i-lucide-box text-3xl text-text-muted" />
            </div>

            <div>
              <div class="flex items-center space-x-3">
                <h2 class="text-2xl font-bold">{{ compStore.currentProject.title }}</h2>
                <MBadge variant="neutral">{{ compStore.currentProject.platform }}</MBadge>
                <MBadge variant="neutral">{{ compStore.currentProject.compType }}</MBadge>
              </div>

              <div class="text-sm text-text-muted mt-1">
                {{ t('comp.project.author', { author: compStore.currentProject.author }) }}
              </div>

              <div class="flex items-center space-x-4 mt-2 text-xs text-text-muted">
                <span>{{ t('comp.project.downloads', { count: compStore.currentProject.downloads.toString() }) }}</span>
                <span>{{ t('comp.project.follows', { count: compStore.currentProject.follows.toString() }) }}</span>
                <span>{{ t('comp.project.updated', { date: formatDate(compStore.currentProject.updatedAt) }) }}</span>
              </div>
            </div>
          </div>

          <div class="flex items-center space-x-3 self-end md:self-auto">
            <MButton
              v-if="compStore.currentProject.websiteUrl"
              variant="tonal"
              @click="openExternalUrl(compStore.currentProject.websiteUrl)"
            >
              <i class="i-lucide-external-link mr-1.5" />
              {{ t('comp.project.website') }}
            </MButton>

            <MIconButton
              :icon="compStore.isFavorite(compStore.currentProject.platform, compStore.currentProject.id) ? 'i-lucide-heart' : 'i-lucide-heart'"
              :class="
                compStore.isFavorite(compStore.currentProject.platform, compStore.currentProject.id)
                  ? 'text-danger hover:opacity-80'
                  : 'text-text-muted hover:text-text'
              "
              @click="compStore.toggleFavorite(compStore.currentProject)"
            />
          </div>
        </div>
      </MCard>

      <!-- Main Layout: Description + Files -->
      <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <!-- Description Column -->
        <div class="lg:col-span-2 space-y-4">
          <h3 class="text-lg font-bold">{{ t('comp.project.description') }}</h3>
          <MCard class="p-6">
            <div
              v-if="renderedDescription"
              class="prose prose-sm dark:prose-invert max-w-none break-words"
              v-html="renderedDescription"
            />
            <div v-else class="text-text-muted py-8 text-center text-sm">
              {{ t('comp.project.noDescription') }}
            </div>
          </MCard>
        </div>

        <!-- Files Column -->
        <div class="space-y-4">
          <div class="flex items-center justify-between">
            <h3 class="text-lg font-bold">{{ t('comp.project.files') }}</h3>
          </div>

          <!-- Filters -->
          <div class="flex items-center space-x-2">
            <div class="flex-1">
              <MSelect
                v-model="selectedMcVersion"
                :options="mcVersionFilterOptions"
              />
            </div>
            <div class="flex-1">
              <MSelect
                v-model="selectedLoader"
                :options="loaderFilterOptions"
              />
            </div>
          </div>

          <!-- Files List -->
          <div class="space-y-3">
            <div v-if="compStore.filesLoading" class="space-y-2 py-4">
              <MSkeleton v-for="n in 3" :key="n" class="h-20 rounded-xl" />
            </div>

            <div
              v-else-if="filteredFiles.length === 0"
              class="py-12"
            >
              <MEmpty :title="t('comp.project.noFiles')" />
            </div>

            <MCard
              v-for="file in filteredFiles"
              :key="file.id"
              class="p-4 space-y-3"
            >
              <div class="flex items-start justify-between">
                <div>
                  <div class="flex items-center space-x-2">
                    <MBadge :variant="getChannelVariant(file.channel)">
                      {{ t(`comp.files.channel.${file.channel}`) }}
                    </MBadge>
                    <span class="font-medium text-sm truncate max-w-[200px]" :title="file.name">
                      {{ file.name }}
                    </span>
                  </div>
                  <div class="text-xs text-text-muted mt-1">
                    {{ formatDate(file.releaseDate) }} • {{ formatFileSize(file.size) }}
                  </div>
                </div>
              </div>

              <div class="flex flex-wrap gap-1">
                <MBadge
                  v-for="v in file.mcVersions"
                  :key="v"
                  variant="neutral"
                >
                  {{ v }}
                </MBadge>
                <MBadge
                  v-for="l in file.loaders"
                  :key="l"
                  variant="neutral"
                >
                  {{ l }}
                </MBadge>
              </div>

              <div class="pt-2 border-t border-outline/10 flex items-center justify-end">
                <div v-if="file.distributionDisabled" class="flex items-center space-x-2">
                  <span class="text-xs text-danger">{{ t('comp.files.distributionDisabled') }}</span>
                  <MButton
                    variant="tonal"
                    size="sm"
                    @click="openExternalUrl(compStore.currentProject?.websiteUrl)"
                  >
                    {{ t('comp.files.openWebsite') }}
                  </MButton>
                </div>
                <MButton
                  v-else
                  variant="primary"
                  size="sm"
                  @click="openDownloadDialog(file)"
                >
                  <i class="i-lucide-download mr-1" />
                  {{ t('comp.files.download') }}
                </MButton>
              </div>
            </MCard>
          </div>
        </div>
      </div>
    </div>

    <!-- Download Instance Selection Dialog -->
    <MDialog
      :open="downloadDialogOpen"
      :title="t('comp.downloadDialog.title')"
      @update:open="downloadDialogOpen = $event"
      @cancel="downloadDialogOpen = false"
    >
      <div class="space-y-4 py-2">
        <div v-if="instancesStore.instances.length === 0" class="text-sm text-danger">
          {{ t('comp.downloadDialog.noInstances') }}
        </div>
        <div v-else class="space-y-3">
          <label class="text-xs font-semibold text-text-muted block">
            {{ t('comp.downloadDialog.selectInstance') }}
          </label>
          <MSelect
            v-model="selectedTargetInstance"
            :options="instanceOptions"
          />
          <div class="text-xs text-text-muted">
            {{ t('comp.downloadDialog.targetSubfolder', { folder: compStore.currentProject?.compType || 'mods' }) }}
          </div>
        </div>
      </div>

      <template #footer>
        <div class="flex items-center justify-end space-x-3">
          <MButton
            variant="text"
            @click="downloadDialogOpen = false"
          >
            {{ t('comp.downloadDialog.cancel') }}
          </MButton>
          <MButton
            variant="primary"
            :loading="downloading"
            :disabled="instancesStore.instances.length === 0 || !selectedTargetInstance"
            @click="startDownload"
          >
            {{ t('comp.downloadDialog.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

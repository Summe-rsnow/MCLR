<script setup lang="ts">
import { onMounted, ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useLaunchStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useAccountsStore } from '../accounts/store'
import { useHomeStore } from '../home/store'
import { useToast } from '../../ui/useToast'
import { ipcClient } from '../../ipc/client'
import {
  MPageHeader,
  MSection,
  MSettingRow,
  MCard,
  MButton,
  MIconButton,
  MSelect,
  MAvatar,
  MBadge,
  MProgress,
  MLoaderIcon,
  MBrandIcon,
  MSwitch,
  MEmpty,
  MSkeletonList,
  getAccountKindMeta,
} from '../../ui'

const { t } = useI18n()
const router = useRouter()
const toast = useToast()

const launchStore = useLaunchStore()
const instancesStore = useInstancesStore()
const accountsStore = useAccountsStore()

const isRepairing = ref(false)

const homeStore = useHomeStore()

function getSavedShowNews(): boolean {
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.getItem === 'function') {
      const val = localStorage.getItem('mclr.launch.showNews')
      return val === 'true'
    }
  } catch {
    // Ignore storage errors in test or restricted environments
  }
  return false
}

const showNews = ref(getSavedShowNews())

async function loadNewsAndAnnouncements(forceRefresh = false) {
  homeStore.loadAnnouncements()
  if (forceRefresh || homeStore.news.length === 0) {
    homeStore.loadNews(homeStore.page || 1, forceRefresh)
  }
}

watch(showNews, (val) => {
  try {
    if (typeof localStorage !== 'undefined' && typeof localStorage.setItem === 'function') {
      localStorage.setItem('mclr.launch.showNews', String(val))
    }
  } catch {
    // Ignore storage errors
  }
  if (val) {
    loadNewsAndAnnouncements()
  }
})

onMounted(async () => {
  await instancesStore.loadSettings()
  await Promise.all([
    instancesStore.fetchInstances(),
    accountsStore.fetchAccounts(),
    launchStore.fetchProcesses(),
  ])
  await launchStore.startListening()
  if (showNews.value) {
    loadNewsAndAnnouncements()
  }
})

const instanceOptions = computed(() => {
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.id,
  }))
})

const accountOptions = computed(() => {
  return accountsStore.accounts.map((a) => ({
    label: `${a.username} (${a.kind})`,
    value: a.id,
  }))
})

const stageLabel = computed(() => {
  return t(`launch.stage.${launchStore.currentStage}`)
})

const accountMeta = computed(() => {
  return getAccountKindMeta(launchStore.selectedAccount?.kind)
})
async function onPickAndSetGameFolder() {
  try {
    const dialog = await import('@tauri-apps/plugin-dialog')
    const selected = await dialog.open({ directory: true })
    if (!selected) return
    const path = Array.isArray(selected) ? selected[0] : selected
    if (!path) return
    const parts = path.replace(/\\/g, '/').split('/').filter(Boolean)
    const name = parts[parts.length - 1] || '.minecraft'
    await instancesStore.addFolder(name, path)
    await instancesStore.fetchInstances()
  } catch (err) {
    console.error('Failed to select game folder:', err)
  }
}

async function onLaunchClick() {
  if (launchStore.isLaunching) return
  try {
    await launchStore.startLaunch()
    toast.show({
      message: t('launch.stage.running'),
      type: 'success',
    })
  } catch (err: unknown) {
    if (launchStore.javaMissingMajor) {
      toast.show({
        message: t('launch.javaMissing.title'),
        type: 'danger',
      })
    } else {
      const msg = err instanceof Error ? err.message : String(err)
      toast.show({
        message: msg,
        type: 'danger',
      })
    }
  }
}

async function onRepairClick() {
  if (!launchStore.selectedInstance) return
  isRepairing.value = true
  try {
    await ipcClient.installRepair(launchStore.selectedInstance.id)
    toast.show({
      message: t('launch.repairSuccess'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  } finally {
    isRepairing.value = false
  }
}

async function onDownloadJavaClick() {
  try {
    const comp =
      launchStore.javaMissingMajor === '8'
        ? 'jre-legacy'
        : launchStore.javaMissingMajor === '21'
          ? 'java-runtime-delta'
          : 'java-runtime-gamma'
    await launchStore.downloadJavaRuntime(comp)
    toast.show({
      message: t('launch.stage.idle'),
      type: 'success',
    })
  } catch (err: unknown) {
    toast.show({
      message: String(err),
      type: 'danger',
    })
  }
}

function onAccountSelectChange(id: string | number) {
  accountsStore.selectAccount(String(id))
}

function onInstanceSelectChange(id: string | number) {
  launchStore.selectInstance(String(id))
}

async function openExternal(url?: string | null) {
  if (!url) return
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}

function handleAnnouncementAction(url?: string | null) {
  if (!url) {
    router.push('/settings/about')
    return
  }
  openExternal(url)
}

function changePage(delta: number) {
  const next = homeStore.page + delta
  if (next >= 1) {
    homeStore.loadNews(next, false)
  }
}
</script>

<template>
  <div class="m-page">
    <!-- Header -->
    <MPageHeader
      :title="t('launch.title')"
      :description="launchStore.activeProcess ? `${t('launch.running')} (PID: ${launchStore.activeProcess.processId})` : undefined"
      icon="i-lucide-rocket"
    >
      <template #actions>
        <MButton
          v-if="launchStore.activeProcess"
          variant="tonal"
          size="sm"
          icon="i-lucide-terminal"
          @click="router.push('/logs')"
        >
          {{ t('launch.runningCard.viewLogs') }}
        </MButton>
        <MButton
          variant="text"
          size="sm"
          icon="i-lucide-sliders-horizontal"
          @click="router.push('/settings/game')"
        >
          {{ t('settingsGame.title') }}
        </MButton>
      </template>
    </MPageHeader>

    <!-- Java Missing Warning Banner -->
    <div
      v-if="launchStore.javaMissingMajor"
      class="bg-danger-soft text-danger-fg border border-danger/20 rounded-xl p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
    >
      <div class="flex items-start gap-3">
        <div class="w-8 h-8 rounded-lg bg-danger/20 text-danger flex items-center justify-center shrink-0">
          <i class="i-lucide-alert-triangle text-lg" />
        </div>
        <div>
          <h4 class="font-semibold text-danger text-sm">
            {{ t('launch.javaMissing.title') }}
          </h4>
          <p class="text-xs text-fg-muted mt-0.5">
            {{ t('launch.javaMissing.message', { major: launchStore.javaMissingMajor }) }}
          </p>
        </div>
      </div>
      <MButton
        variant="danger"
        size="sm"
        :loading="launchStore.isDownloadingJava"
        icon="i-lucide-download"
        class="shrink-0"
        @click="onDownloadJavaClick"
      >
        {{ t('launch.javaMissing.downloadAction') }}
      </MButton>
    </div>

    <!-- Hero Card -->
    <MCard variant="tinted" padding="lg" class="relative">
      <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
        <div class="flex items-center gap-5 min-w-0">
          <MLoaderIcon
            :kind="launchStore.selectedInstance?.kind || (launchStore.selectedInstance?.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
            :icon="launchStore.selectedInstance?.config?.icon"
            :size="56"
          />
          <div class="min-w-0 space-y-1.5">
            <div class="flex items-center gap-2">
              <h2 class="text-2xl font-bold text-fg truncate">
                {{ instancesStore.instances.length === 0 ? t('instances.empty.title') : (launchStore.selectedInstance?.name || t('launch.noInstance')) }}
              </h2>
              <MButton
                v-if="launchStore.selectedInstance"
                variant="text"
                size="xs"
                icon="i-lucide-wrench"
                :loading="isRepairing"
                @click="onRepairClick"
              >
                {{ t('launch.repairInstance') }}
              </MButton>
            </div>
            <div v-if="launchStore.selectedInstance" class="flex flex-wrap items-center gap-2">
              <MBadge variant="primary">
                {{ launchStore.selectedInstance.mcVersion }}
              </MBadge>
              <MBadge
                v-if="launchStore.selectedInstance.loaders.length > 0"
                variant="neutral"
              >
                {{ launchStore.selectedInstance.loaders[0] }}
              </MBadge>
              <MBadge v-else variant="neutral">
                Vanilla
              </MBadge>
            </div>
          </div>
        </div>

        <!-- Launch CTA inside hero -->
        <div class="flex flex-col items-center sm:items-end w-full md:w-auto shrink-0 gap-2">
          <!-- Warning / Guide prompt if no game folder configured -->
          <div
            v-if="!instancesStore.currentFolder"
            class="flex items-center gap-2 p-2 rounded-xl bg-warning/10 border border-warning/20 text-xs text-warning"
            data-testid="launch-no-folder-warning"
          >
            <span class="w-4 h-4 i-lucide-alert-triangle shrink-0" />
            <span>{{ t('launch.noFolderWarning') }}</span>
            <MButton
              variant="tonal"
              size="xs"
              icon="i-lucide-folder-plus"
              @click="onPickAndSetGameFolder"
            >
              {{ t('launch.selectFolder') }}
            </MButton>
          </div>

          <MButton
            v-if="instancesStore.instances.length === 0"
            size="lg"
            icon="i-lucide-download"
            variant="primary"
            class="h-14 px-10 rounded-2xl text-lg font-bold m-refract w-full sm:w-auto shadow-2"
            @click="router.push('/download')"
          >
            {{ t('launch.button.download') }}
          </MButton>
          <MButton
            v-else
            size="lg"
            icon="i-lucide-play"
            class="h-14 px-10 rounded-2xl text-lg font-bold m-refract w-full sm:w-auto shadow-2"
            :loading="launchStore.isLaunching"
            :disabled="!instancesStore.currentFolder || !launchStore.selectedInstance || !launchStore.selectedAccount"
            @click="onLaunchClick"
          >
            <span v-if="launchStore.isLaunching">{{ stageLabel }}</span>
            <span v-else>{{ t('launch.launchButton') }}</span>
          </MButton>
          <div v-if="launchStore.isLaunching" class="w-full sm:w-56 flex flex-col gap-1 items-center sm:items-end">
            <MProgress :value="launchStore.launchProgress" size="sm" class="w-full" />
            <span class="text-xs text-fg-muted font-mono">{{ stageLabel }}</span>
          </div>
        </div>
      </div>
    </MCard>

    <!-- Selection Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <!-- Instance Selection Section -->
      <MSection :title="t('launch.selectInstance')" icon="i-lucide-box">
        <template #actions>
          <MButton
            variant="text"
            size="xs"
            icon="i-lucide-plus"
            @click="router.push('/download')"
          >
            {{ t('launch.createInstance') }}
          </MButton>
        </template>
        <div class="space-y-4">
          <MSettingRow :label="t('launch.selectInstance')">
            <MSelect
              v-if="instancesStore.instances.length > 0"
              :model-value="launchStore.selectedInstance?.id || ''"
              :options="instanceOptions"
              size="sm"
              class="w-56"
              @update:model-value="onInstanceSelectChange"
            />
            <MButton
              v-else
              variant="tonal"
              size="sm"
              icon="i-lucide-plus"
              @click="router.push('/download')"
            >
              {{ t('launch.createInstance') }}
            </MButton>
          </MSettingRow>
        </div>
      </MSection>

      <!-- Account Selection Section -->
      <MSection :title="t('launch.selectAccount')" icon="i-lucide-user-round">
        <template #actions>
          <MButton
            variant="text"
            size="xs"
            icon="i-lucide-user-plus"
            @click="router.push('/accounts')"
          >
            {{ t('launch.addAccount') }}
          </MButton>
        </template>
        <div class="space-y-4">
          <!-- Account Preview Card -->
          <MCard v-if="launchStore.selectedAccount" variant="well" padding="sm" class="flex items-center gap-3">
            <MAvatar
              :name="launchStore.selectedAccount.username"
              :src="launchStore.selectedAccount.skinHeadDataUrl || undefined"
              :size="36"
            />
            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-1.5">
                <span class="font-semibold text-sm text-fg truncate">
                  {{ launchStore.selectedAccount.username }}
                </span>
                <MBrandIcon
                  v-if="accountMeta.brand"
                  :name="accountMeta.brand"
                  class="w-3.5 h-3.5 text-primary shrink-0"
                />
                <i
                  v-else
                  :class="[accountMeta.icon, 'text-xs text-fg-muted shrink-0']"
                />
              </div>
              <div class="flex items-center gap-2 text-xs text-fg-muted">
                <span class="capitalize">{{ launchStore.selectedAccount.kind }}</span>
                <span v-if="launchStore.selectedAccount.serverName" class="truncate">
                  • {{ launchStore.selectedAccount.serverName }}
                </span>
              </div>
            </div>
          </MCard>

          <MSettingRow :label="t('launch.selectAccount')">
            <MSelect
              v-if="accountsStore.accounts.length > 0"
              :model-value="launchStore.selectedAccount?.id || ''"
              :options="accountOptions"
              size="sm"
              class="w-56"
              @update:model-value="onAccountSelectChange"
            />
            <MButton
              v-else
              variant="tonal"
              size="sm"
              icon="i-lucide-user-plus"
              @click="router.push('/accounts')"
            >
              {{ t('launch.addAccount') }}
            </MButton>
          </MSettingRow>
        </div>
      </MSection>
    </div>

    <!-- News & Announcements Section -->
    <div class="space-y-4 pt-2">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <span class="i-lucide-newspaper w-5 h-5 text-primary" />
          <h3 class="text-base font-semibold text-fg">
            {{ t('launch.news.title') }}
          </h3>
        </div>
        <div class="flex items-center gap-3">
          <MButton
            v-if="showNews"
            data-testid="news-refresh-btn"
            size="xs"
            variant="tonal"
            icon="i-lucide-refresh-cw"
            :loading="homeStore.loadingNews"
            @click="loadNewsAndAnnouncements(true)"
          >
            {{ homeStore.loadingNews ? t('launch.news.refreshing') : t('launch.news.refresh') }}
          </MButton>
          <div class="flex items-center gap-2">
            <span class="text-xs text-fg-muted select-none">{{ t('launch.news.toggleLabel') }}</span>
            <MSwitch v-model="showNews" data-testid="news-toggle-switch" size="sm" />
          </div>
        </div>
      </div>

      <!-- Off State: Compact Card -->
      <MCard
        v-if="!showNews"
        data-testid="news-compact-off-card"
        variant="well"
        padding="md"
        class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
      >
        <div class="flex items-center gap-3 min-w-0">
          <div class="w-9 h-9 rounded-xl bg-sunken flex items-center justify-center shrink-0 text-fg-muted">
            <span class="i-lucide-newspaper w-4.5 h-4.5" />
          </div>
          <div class="min-w-0">
            <p class="text-xs text-fg-muted">
              {{ t('launch.news.disabledHint') }}
            </p>
          </div>
        </div>
        <MButton
          data-testid="news-enable-btn"
          size="xs"
          variant="tonal"
          @click="showNews = true"
        >
          {{ t('launch.news.enableAction') }}
        </MButton>
      </MCard>

      <!-- On State: Announcements & News List -->
      <div v-else class="space-y-4" data-testid="news-container-on">
        <!-- Announcements Banner -->
        <div v-if="homeStore.announcements.length > 0" class="flex flex-col gap-2">
          <div
            v-for="item in homeStore.announcements"
            :key="item.id"
            class="p-4 rounded-xl border border-primary/20 bg-primary-soft text-primary-soft-fg flex flex-col md:flex-row md:items-center justify-between gap-3"
          >
            <div class="flex items-start gap-3">
              <span class="i-lucide-megaphone w-5 h-5 text-primary shrink-0 mt-0.5" />
              <div class="flex flex-col gap-1">
                <span class="text-sm font-semibold text-fg">{{ item.title }}</span>
                <p class="text-xs text-fg-muted leading-relaxed">{{ item.detail }}</p>
              </div>
            </div>
            <div class="flex items-center gap-2 shrink-0 self-end md:self-auto">
              <MButton
                v-if="item.btn1"
                size="sm"
                variant="tonal"
                @click="handleAnnouncementAction(item.btn1.url)"
              >
                {{ item.btn1.text }}
              </MButton>
              <MButton
                v-if="item.btn2"
                size="sm"
                variant="tonal"
                @click="handleAnnouncementAction(item.btn2.url)"
              >
                {{ item.btn2.text }}
              </MButton>
              <MIconButton
                icon="i-lucide-x"
                size="sm"
                variant="text"
                :tooltip="t('launch.announcements.dismiss')"
                @click="homeStore.dismissAnnouncement(item.id)"
              />
            </div>
          </div>
        </div>

        <!-- News Loading Skeleton -->
        <div v-if="homeStore.loadingNews && homeStore.news.length === 0" class="p-1">
          <MSkeletonList :rows="3" variant="card" />
        </div>

        <!-- News Error -->
        <div v-else-if="homeStore.newsError && homeStore.news.length === 0" class="py-8 flex items-center justify-center">
          <MEmpty :description="t('launch.news.fetchFailed')">
            <MButton size="sm" @click="loadNewsAndAnnouncements(true)">
              {{ t('launch.news.refresh') }}
            </MButton>
          </MEmpty>
        </div>

        <!-- News Empty -->
        <div v-else-if="homeStore.news.length === 0" class="py-8 flex items-center justify-center">
          <MEmpty :description="t('launch.news.empty')" />
        </div>

        <!-- News Grid -->
        <div v-else class="space-y-4">
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 items-start">
            <MCard
              v-for="(item, index) in homeStore.news"
              :key="item.id"
              hoverable
              class="flex flex-col cursor-pointer overflow-hidden group m-stagger"
              :style="{ '--i': index }"
              @click="openExternal(item.articleUrl)"
            >
              <!-- Cover Image -->
              <div class="w-full h-36 bg-sunken relative overflow-hidden shrink-0">
                <img
                  v-if="item.imageUrl"
                  :src="item.imageUrl"
                  alt="cover"
                  class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                  loading="lazy"
                  decoding="async"
                />
                <div v-else class="w-full h-full flex items-center justify-center text-fg-muted/40">
                  <span class="i-lucide-newspaper w-10 h-10" />
                </div>
                <div class="absolute top-2 left-2">
                  <MBadge variant="primary">{{ item.category }}</MBadge>
                </div>
              </div>

              <!-- Content -->
              <div class="p-4 flex-1 flex flex-col justify-between gap-2">
                <div class="flex flex-col gap-1.5">
                  <div class="flex items-center justify-between text-xs text-fg-muted">
                    <span>{{ item.date }}</span>
                  </div>
                  <h4 class="text-sm font-semibold text-fg line-clamp-2 leading-snug group-hover:text-primary transition-colors">
                    {{ item.title }}
                  </h4>
                  <p class="text-xs text-fg-muted line-clamp-3 leading-relaxed">
                    {{ item.summary }}
                  </p>
                </div>

                <div class="pt-2 flex items-center text-xs text-primary font-medium gap-1 self-end">
                  <span>{{ t('launch.news.readMore') }}</span>
                  <span class="i-lucide-arrow-right w-3.5 h-3.5" />
                </div>
              </div>
            </MCard>
          </div>

          <!-- Pagination Bar -->
          <div class="flex items-center justify-between pt-3 border-t border-line shrink-0">
            <MButton
              size="sm"
              variant="tonal"
              :disabled="homeStore.page <= 1 || homeStore.loadingNews"
              @click="changePage(-1)"
            >
              {{ t('launch.news.pagePrev') }}
            </MButton>
            <span class="text-xs text-fg-muted">
              {{ t('launch.news.pageNumber', { page: homeStore.page }) }}
            </span>
            <MButton
              size="sm"
              variant="tonal"
              :disabled="homeStore.news.length === 0 || homeStore.loadingNews"
              @click="changePage(1)"
            >
              {{ t('launch.news.pageNext') }}
            </MButton>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

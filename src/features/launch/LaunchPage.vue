<script setup lang="ts">
import { onMounted, ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useLaunchStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useAccountsStore } from '../accounts/store'
import { useHomeStore } from '../home/store'
import { MSection, MButton, MSelect, getInstanceIconMeta, useToast } from '../../ui'
import InstanceContextMenu from '../instances/components/InstanceContextMenu.vue'

const toast = useToast()
import LaunchHeroBanner from './LaunchHeroBanner.vue'
import InstancePreviewCard from './InstancePreviewCard.vue'
import AccountPreviewCard from './AccountPreviewCard.vue'
import LaunchNewsSection from './LaunchNewsSection.vue'

const { t } = useI18n()
const router = useRouter()

const launchStore = useLaunchStore()
const instancesStore = useInstancesStore()
const accountsStore = useAccountsStore()
const homeStore = useHomeStore()

const isRepairing = ref(false)

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
  return instancesStore.instances.map((i) => {
    const kind = i.kind || (i.loaders?.[0]?.toLowerCase() ?? 'vanilla')
    const meta = getInstanceIconMeta(i.config?.icon, kind)
    return {
      label: i.name,
      value: i.id,
      avatar: meta.image ?? undefined,
      icon: meta.icon,
      badge: i.mcVersion || undefined,
    }
  })
})

const accountOptions = computed(() => {
  return accountsStore.accounts.map((a) => ({
    label: a.username,
    value: a.id,
    avatar: a.skinHeadDataUrl || undefined,
    badge: t('accounts.types.' + a.kind),
  }))
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
    toast.danger(String(err))
  }
}

function onAccountSelectChange(id: string | number) {
  accountsStore.selectAccount(String(id))
}

function onInstanceSelectChange(id: string | number) {
  launchStore.selectInstance(String(id))
}

// Right-click menu for the instance cards on this page (same component as the
// instances page, limited to the non-destructive entries).
const contextMenuOpen = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)

function onInstanceContextMenu(ev: MouseEvent, instance: { id: string }) {
  ev.preventDefault()
  launchStore.selectInstance(instance.id)
  contextMenuX.value = ev.clientX
  contextMenuY.value = ev.clientY
  contextMenuOpen.value = true
}

function handleContextAction(key: string, instance: { id: string }) {
  if (key === 'manage') {
    router.push('/instances/' + encodeURIComponent(instance.id))
  } else if (key === 'openFolder') {
    void instancesStore.openFolder(instance.id)
  } else if (key === 'star') {
    void instancesStore.toggleStar(instance.id)
  }
}
</script>

<template>
  <div class="m-page">
    <!-- Hero Banner (Title, actions, java warning, hero card & launch CTA) -->
    <LaunchHeroBanner
      v-model:is-repairing="isRepairing"
      @pick-folder="onPickAndSetGameFolder"
      @contextmenu="(ev) => onInstanceContextMenu(ev, launchStore.selectedInstance!)"
    />

    <!-- Selection Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
      <!-- Instance Selection Section -->
      <MSection :title="t('launch.selectInstance')" icon="i-lucide-box">
        <template #actions>
          <MButton variant="text" size="xs" icon="i-lucide-plus" @click="router.push('/download')">
            {{ t('launch.createInstance') }}
          </MButton>
        </template>
        <div class="space-y-4">
          <!-- Instance Preview Card -->
          <InstancePreviewCard
            v-if="launchStore.selectedInstance"
            :instance="launchStore.selectedInstance"
            @contextmenu="onInstanceContextMenu"
          />

          <!-- Stacked + full width: these panels are half of a 2-column grid, so a
               fixed-width row control overflows the card on narrow windows. -->
          <div class="flex flex-col gap-1.5">
            <span class="text-sm font-medium text-fg">{{ t('launch.selectInstance') }}</span>
            <MSelect
              v-if="instancesStore.instances.length > 0"
              :model-value="launchStore.selectedInstance?.id || ''"
              :options="instanceOptions"
              size="sm"
              class="w-full"
              data-testid="launch-instance-select"
              @update:model-value="onInstanceSelectChange"
            />
            <p v-else class="text-sm text-fg-muted py-1">
              {{ t('launch.noInstanceHint') }}
            </p>
          </div>
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
          <AccountPreviewCard
            v-if="launchStore.selectedAccount"
            :account="launchStore.selectedAccount"
          />

          <div class="flex flex-col gap-1.5">
            <span class="text-sm font-medium text-fg">{{ t('launch.selectAccount') }}</span>
            <MSelect
              v-if="accountsStore.accounts.length > 0"
              :model-value="launchStore.selectedAccount?.id || ''"
              :options="accountOptions"
              size="sm"
              class="w-full"
              data-testid="launch-account-select"
              @update:model-value="onAccountSelectChange"
            />
            <p v-else class="text-sm text-fg-muted py-1">
              {{ t('launch.noAccountHint') }}
            </p>
          </div>
        </div>
      </MSection>
    </div>

    <InstanceContextMenu
      v-model:open="contextMenuOpen"
      :instance="launchStore.selectedInstance"
      compact
      :x="contextMenuX"
      :y="contextMenuY"
      @action="handleContextAction"
    />

    <!-- News & Announcements Section -->
    <LaunchNewsSection v-model:show-news="showNews" @refresh="loadNewsAndAnnouncements" />
  </div>
</template>

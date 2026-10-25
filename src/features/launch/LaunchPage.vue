<script setup lang="ts">
import { onMounted, ref, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useLaunchStore } from './store'
import { useInstancesStore } from '../instances/store'
import { useAccountsStore } from '../accounts/store'
import { useHomeStore } from '../home/store'
import { MSection, MSettingRow, MButton, MSelect } from '../../ui'
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
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.id,
  }))
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
    console.error('Failed to select game folder:', err)
  }
}

function onAccountSelectChange(id: string | number) {
  accountsStore.selectAccount(String(id))
}

function onInstanceSelectChange(id: string | number) {
  launchStore.selectInstance(String(id))
}
</script>

<template>
  <div class="m-page">
    <!-- Hero Banner (Title, actions, java warning, hero card & launch CTA) -->
    <LaunchHeroBanner v-model:is-repairing="isRepairing" @pick-folder="onPickAndSetGameFolder" />

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
          />

          <MSettingRow :label="t('launch.selectInstance')">
            <MSelect
              v-if="instancesStore.instances.length > 0"
              :model-value="launchStore.selectedInstance?.id || ''"
              :options="instanceOptions"
              size="sm"
              class="w-56"
              @update:model-value="onInstanceSelectChange"
            />
            <p v-else class="text-sm text-fg-muted py-1">
              {{ t('launch.noInstanceHint') }}
            </p>
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
          <AccountPreviewCard
            v-if="launchStore.selectedAccount"
            :account="launchStore.selectedAccount"
          />

          <MSettingRow :label="t('launch.selectAccount')">
            <MSelect
              v-if="accountsStore.accounts.length > 0"
              :model-value="launchStore.selectedAccount?.id || ''"
              :options="accountOptions"
              size="sm"
              class="w-56"
              @update:model-value="onAccountSelectChange"
            />
            <p v-else class="text-sm text-fg-muted py-1">
              {{ t('launch.noAccountHint') }}
            </p>
          </MSettingRow>
        </div>
      </MSection>
    </div>

    <!-- News & Announcements Section -->
    <LaunchNewsSection v-model:show-news="showNews" @refresh="loadNewsAndAnnouncements" />
  </div>
</template>

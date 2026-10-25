<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import type { ServerEntry, ServerPingResult, InstanceInfo } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { useInstancesStore } from '../../instances/store'
import {
  MCard,
  MButton,
  MIconButton,
  MBadge,
  MDialog,
  MInput,
  MSelect,
  MEmpty,
  MSkeletonList,
  useToast,
  useDialog,
} from '../../../ui'
const props = defineProps<{
  instanceId: string
  instance?: InstanceInfo
}>()

const { t } = useI18n()
const router = useRouter()
const toast = useToast()
const { confirm } = useDialog()
const instancesStore = useInstancesStore()

const servers = ref<ServerEntry[]>([])
const loading = ref(false)
const pingResults = ref<Record<string, ServerPingResult>>({})
const pinging = ref<Record<string, boolean>>({})

// Add/Edit Dialog
const showEditDialog = ref(false)
const editingIndex = ref<number | null>(null)
const serverForm = ref({
  name: '',
  ip: '',
  acceptTextures: '0',
})
const savingServer = ref(false)

const acceptTexturesOptions = [
  { label: t('instanceDetail.servers.acceptTexturesPrompt'), value: '0' },
  { label: t('instanceDetail.servers.acceptTexturesEnabled'), value: '1' },
  { label: t('instanceDetail.servers.acceptTexturesDisabled'), value: '2' },
]

async function fetchServers() {
  loading.value = true
  try {
    servers.value = await ipcClient.serversList(props.instanceId)
    pingAllServers()
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    loading.value = false
  }
}

let pingCancelled = false

async function pingServer(ip: string) {
  if (pingCancelled || pinging.value[ip]) return
  pinging.value[ip] = true
  try {
    const res = await ipcClient.serverPing(ip)
    if (!pingCancelled) {
      pingResults.value[ip] = res
    }
  } catch {
    if (!pingCancelled) {
      pingResults.value[ip] = {
        ip,
        resolvedAddr: '',
        online: false,
        motd: '',
        versionName: '',
        protocol: 0,
        playersOnline: 0,
        playersMax: 0,
        favicon: null,
        latencyMs: 999,
      }
    }
  } finally {
    pinging.value[ip] = false
  }
}

async function pingAllServers() {
  pingCancelled = false
  const ips = servers.value.map((s) => s.ip).filter(Boolean)
  if (ips.length === 0) return

  const CONCURRENCY = 4
  let currentIndex = 0

  async function worker() {
    while (currentIndex < ips.length && !pingCancelled) {
      const ip = ips[currentIndex++]
      if (ip) {
        await pingServer(ip)
      }
    }
  }

  const workers = Array.from({ length: Math.min(CONCURRENCY, ips.length) }, () => worker())
  await Promise.all(workers)
}

onUnmounted(() => {
  pingCancelled = true
})

function openAddDialog() {
  editingIndex.value = null
  serverForm.value = {
    name: '',
    ip: '',
    acceptTextures: '0',
  }
  showEditDialog.value = true
}

function openEditDialog(server: ServerEntry) {
  editingIndex.value = server.index
  serverForm.value = {
    name: server.name,
    ip: server.ip,
    acceptTextures: server.acceptTextures !== null && server.acceptTextures !== undefined
      ? String(server.acceptTextures)
      : '0',
  }
  showEditDialog.value = true
}

async function saveServer() {
  const name = serverForm.value.name.trim()
  const ip = serverForm.value.ip.trim()
  if (!name || !ip) return

  savingServer.value = true
  try {
    const texturesVal = serverForm.value.acceptTextures === '0'
      ? null
      : Number(serverForm.value.acceptTextures)
    if (editingIndex.value === null) {
      servers.value = await ipcClient.serverAdd(
        props.instanceId,
        name,
        ip,
        texturesVal,
      )
    } else {
      servers.value = await ipcClient.serverUpdate(
        props.instanceId,
        editingIndex.value,
        name,
        ip,
        texturesVal,
      )
    }
    toast.success(t('instanceDetail.servers.saveSuccess'))
    showEditDialog.value = false
    pingServer(ip)
  } catch (e: unknown) {
    toast.danger(String(e))
  } finally {
    savingServer.value = false
  }
}

async function handleDelete(server: ServerEntry) {
  const ok = await confirm({
    title: t('instanceDetail.servers.deleteConfirmTitle'),
    content: t('instanceDetail.servers.deleteConfirm', { name: server.name }),
  })
  if (!ok) return

  try {
    servers.value = await ipcClient.serverRemove(props.instanceId, server.index)
    toast.success(t('instanceDetail.servers.deleteSuccess'))
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

async function launchIntoServer(server: ServerEntry) {
  try {
    // 1. Get existing instance config or fallback
    const targetInst = props.instance || instancesStore.instances.find((i) => i.id === props.instanceId)
    const currentConfig = targetInst?.config || {
      icon: 'grass_block',
      category: 'Default',
      pinned: false,
      isolation: 'global',
      loginRequirement: 'any',
    }

    // 2. Set serverQuickPlay
    await ipcClient.instanceUpdateConfig(props.instanceId, {
      ...currentConfig,
      serverQuickPlay: server.ip,
    })
    await instancesStore.fetchInstances()

    // 3. Navigate to launch page
    toast.success(t('instanceDetail.servers.launchDirectHint'))
    router.push('/launch')
  } catch (e: unknown) {
    toast.danger(String(e))
  }
}

function getLatencyBadgeVariant(ms: number): 'success' | 'warning' | 'danger' {
  if (ms < 80) return 'success'
  if (ms < 180) return 'warning'
  return 'danger'
}

onMounted(() => {
  fetchServers()
})
</script>

<template>
  <div class="max-w-4xl space-y-4">
    <!-- Top Bar -->
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <h2 class="text-base font-bold text-fg">
          {{ t('instanceDetail.tabs.servers') }}
        </h2>
        <span class="text-xs text-fg-muted font-mono">
          ({{ servers.length }})
        </span>
      </div>
      <div class="flex items-center gap-2">
        <MButton variant="primary" size="sm" @click="openAddDialog">
          <span class="w-4 h-4 i-lucide-plus mr-1" />
          {{ t('instanceDetail.servers.add') }}
        </MButton>
        <MButton variant="tonal" size="sm" :loading="loading" @click="pingAllServers">
          <span class="w-4 h-4 i-lucide-activity mr-1" />
          {{ t('instanceDetail.servers.pingAll') }}
        </MButton>
      </div>
    </div>

    <!-- Skeleton Loading -->
    <div v-if="loading && servers.length === 0" class="py-4">
      <MSkeletonList :rows="3" variant="card" />
    </div>

    <!-- Empty State -->
    <div v-else-if="servers.length === 0" class="py-16">
      <MEmpty
        :title="t('instanceDetail.servers.empty')"
        icon="i-lucide-server"
      />
    </div>
    <!-- Server List -->
    <div v-else class="space-y-3">
      <MCard
        v-for="server in servers"
        :key="server.index"
        variant="well"
        hoverable
        padding="md"
        class="flex flex-col md:flex-row items-start md:items-center justify-between gap-4 transition-all"
      >
        <div class="flex items-start gap-4 min-w-0">
          <!-- Server Icon -->
          <div class="w-14 h-14 rounded-xl bg-sunken/60 flex items-center justify-center shrink-0 border border-line overflow-hidden text-primary">
            <img
              v-if="pingResults[server.ip]?.favicon || server.icon"
              :src="pingResults[server.ip]?.favicon || server.icon || ''"
              class="w-full h-full object-cover"
              alt="Server Favicon"
             loading="lazy" decoding="async"/>
            <span v-else class="w-7 h-7 i-lucide-server" />
          </div>

          <!-- Metadata & Ping Status -->
          <div class="min-w-0 space-y-1">
            <div class="flex items-center gap-2.5">
              <span class="text-base font-bold text-fg truncate">
                {{ server.name }}
              </span>
              <MBadge v-if="pinging[server.ip]" variant="neutral" size="sm" class="animate-pulse">
                <span class="inline-block w-1.5 h-1.5 rounded-full bg-primary mr-1" />
                Ping...
              </MBadge>
              <template v-else-if="pingResults[server.ip]">
                <MBadge
                  v-if="pingResults[server.ip].online"
                  :variant="getLatencyBadgeVariant(pingResults[server.ip].latencyMs)"
                  size="sm"
                >
                  {{ pingResults[server.ip].latencyMs }} ms
                </MBadge>
                <MBadge v-else variant="danger" size="sm">
                  {{ t('instanceDetail.servers.offline') }}
                </MBadge>
              </template>
            </div>
            <div class="text-xs text-fg-muted font-mono">
              {{ server.ip }}
            </div>

            <div
              v-if="pingResults[server.ip]?.online"
              class="text-xs text-fg-muted/80 line-clamp-1"
            >
              {{ pingResults[server.ip].motd }}
            </div>

            <div
              v-if="pingResults[server.ip]?.online"
              class="flex flex-wrap items-center gap-x-4 text-[11px] text-fg-muted"
            >
              <span>
                {{ t('instanceDetail.servers.players') }}:
                <span class="font-mono font-medium text-fg">
                  {{ pingResults[server.ip].playersOnline }}/{{ pingResults[server.ip].playersMax }}
                </span>
              </span>
              <span>
                {{ t('instanceDetail.servers.version') }}:
                <span class="font-mono">{{ pingResults[server.ip].versionName }}</span>
              </span>
            </div>
          </div>
        </div>

        <!-- Actions -->
        <div class="flex items-center gap-2 self-end md:self-center shrink-0">
          <MButton
            variant="primary"
            size="sm"
            @click="launchIntoServer(server)"
          >
            <span class="w-4 h-4 i-lucide-play mr-1" />
            {{ t('instanceDetail.servers.launchDirect') }}
          </MButton>

          <MIconButton
            icon="i-lucide-refresh-cw"
            size="sm"
            :loading="pinging[server.ip]"
            :title="t('instanceDetail.servers.refresh')"
            @click="pingServer(server.ip)"
          />

          <MIconButton
            icon="i-lucide-edit-2"
            size="sm"
            :title="t('instanceDetail.servers.edit')"
            @click="openEditDialog(server)"
          />

          <MIconButton
            icon="i-lucide-trash-2"
            size="sm"
            class="text-danger hover:bg-danger/10"
            :title="t('instanceDetail.servers.delete')"
            @click="handleDelete(server)"
          />
        </div>
      </MCard>
    </div>

    <!-- Add/Edit Server Dialog -->
    <MDialog
      :open="showEditDialog"
      :title="editingIndex === null ? t('instanceDetail.servers.addTitle') : t('instanceDetail.servers.editTitle')"
      @update:open="showEditDialog = $event"
    >
      <div class="py-2 space-y-4 min-w-80">
        <MInput
          v-model="serverForm.name"
          :label="t('instanceDetail.servers.name')"
          :placeholder="t('instanceDetail.servers.namePlaceholder')"
          autofocus
        />

        <MInput
          v-model="serverForm.ip"
          :label="t('instanceDetail.servers.ip')"
          :placeholder="t('instanceDetail.servers.ipPlaceholder')"
          @keydown.enter="saveServer"
        />

        <div class="space-y-1.5">
          <label class="text-sm font-semibold text-fg">
            {{ t('instanceDetail.servers.acceptTextures') }}
          </label>
          <MSelect
            v-model="serverForm.acceptTextures"
            :options="acceptTexturesOptions"
          />
        </div>
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="text" @click="showEditDialog = false">
            {{ t('common.cancel') }}
          </MButton>
          <MButton
            variant="primary"
            :loading="savingServer"
            @click="saveServer"
          >
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

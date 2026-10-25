<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import {
  ipcClient,
  type CompFile,
  type ExtraMod,
  type InstallMode,
  type InstallRequest,
  type InstanceInfo,
  type LoaderKind,
  type LoaderVersionEntry,
} from '../../ipc/client'
import {
  MButton,
  MCard,
  MInput,
  MSelect,
  MBadge,
  MSwitch,
  MSkeleton,
  useToast,
} from '../../ui'
import {
  defaultName,
  dropOf,
  loaderIncompatibilities,
  loaderVisibility,
  validateInstanceNameLocal,
} from './compat'

const props = withDefaults(
  defineProps<{
    mode?: 'create' | 'modify'
    mcVersion: string
    instance?: InstanceInfo
  }>(),
  {
    mode: 'create',
  },
)

const emit = defineEmits<{
  (e: 'back'): void
  (e: 'success', taskId: string): void
}>()

const { t } = useI18n()
const toast = useToast()
const router = useRouter()

// Primary loader state
type PrimaryLoaderId =
  | 'none'
  | 'fabric'
  | 'neoforge'
  | 'forge'
  | 'cleanroom'
  | 'legacyFabric'
  | 'labymod'

const selectedPrimary = ref<PrimaryLoaderId>('none')
const primaryVersions = ref<LoaderVersionEntry[]>([])
const selectedPrimaryVersion = ref('')
const loadingPrimaryVersions = ref(false)

// Additional loaders
const optifineEnabled = ref(false)
const optifineVersions = ref<LoaderVersionEntry[]>([])
const selectedOptifineVersion = ref('')
const loadingOptifineVersions = ref(false)

const liteloaderEnabled = ref(false)
const liteloaderVersions = ref<LoaderVersionEntry[]>([])
const selectedLiteloaderVersion = ref('')
const loadingLiteloaderVersions = ref(false)

// Add-ons
const fabricApiCandidates = ref<CompFile[]>([])
const fabricApiEnabled = ref(true)
const legacyFabricApiCandidates = ref<CompFile[]>([])
const legacyFabricApiEnabled = ref(true)
const optiFabricCandidates = ref<CompFile[]>([])
const optiFabricEnabled = ref(true)
const loadingExtras = ref(false)

// Incompatibilities & Visibility
const visibility = computed(() => loaderVisibility(props.mcVersion))
const incomp = computed(() =>
  loaderIncompatibilities(props.mcVersion, {
    primaryLoader:
      selectedPrimary.value === 'none'
        ? null
        : (selectedPrimary.value as Exclude<PrimaryLoaderId, 'none'>),
    optifine: optifineEnabled.value,
    liteloader: liteloaderEnabled.value,
  }),
)

// Instance Name (Create mode)
const instanceName = ref('')
const isNameManuallyEdited = ref(false)
const nameValidationError = ref<string | null>(null)
const submitting = ref(false)

// Modify mode state
const initialSummary = ref('')
const installedModFiles = ref<string[]>([])
const isQuiltInstance = computed(() => {
  return (
    props.instance?.loaders?.some((l) =>
      l.toLowerCase().includes('quilt'),
    ) ?? false
  )
})

// Primary loaders list definition
interface LoaderDef {
  id: PrimaryLoaderId
  label: string
  visible: boolean
  icon: string
}

const primaryLoaderDefs = computed<LoaderDef[]>(() => [
  { id: 'none', label: t('download.configurator.vanillaPure'), visible: true, icon: 'i-lucide-box' },
  {
    id: 'fabric',
    label: 'Fabric',
    visible: visibility.value.fabric,
    icon: 'i-lucide-layers',
  },
  {
    id: 'neoforge',
    label: 'NeoForge',
    visible: visibility.value.neoforge,
    icon: 'i-lucide-flame',
  },
  {
    id: 'forge',
    label: 'Forge',
    visible: visibility.value.forge,
    icon: 'i-lucide-anvil',
  },
  {
    id: 'cleanroom',
    label: 'Cleanroom',
    visible: visibility.value.cleanroom,
    icon: 'i-lucide-cpu',
  },
  {
    id: 'legacyFabric',
    label: 'LegacyFabric',
    visible: visibility.value.legacyFabric,
    icon: 'i-lucide-archive',
  },
  {
    id: 'labymod',
    label: 'LabyMod',
    visible: visibility.value.labymod,
    icon: 'i-lucide-sparkles',
  },
])

// Auto naming calculation
function updateAutoName() {
  if (props.mode === 'modify' || isNameManuallyEdited.value) return
  const name = defaultName({
    mcVersion: props.mcVersion,
    fabricVersion:
      selectedPrimary.value === 'fabric'
        ? selectedPrimaryVersion.value
        : undefined,
    legacyFabricVersion:
      selectedPrimary.value === 'legacyFabric'
        ? selectedPrimaryVersion.value
        : undefined,
    forgeVersion:
      selectedPrimary.value === 'forge'
        ? selectedPrimaryVersion.value
        : undefined,
    neoforgeVersion:
      selectedPrimary.value === 'neoforge'
        ? selectedPrimaryVersion.value
        : undefined,
    cleanroomVersion:
      selectedPrimary.value === 'cleanroom'
        ? selectedPrimaryVersion.value
        : undefined,
    liteloader: liteloaderEnabled.value,
    optifineDisplay: optifineEnabled.value
      ? selectedOptifineVersion.value
      : undefined,
  })
  instanceName.value = name
  validateName()
}

function validateName() {
  if (props.mode === 'modify') {
    nameValidationError.value = null
    return
  }
  const localErr = validateInstanceNameLocal(instanceName.value)
  if (localErr) {
    nameValidationError.value = localErr
  } else {
    nameValidationError.value = null
  }
}

function handleNameInput(val: string) {
  instanceName.value = val
  isNameManuallyEdited.value = true
  validateName()
}

function resetToAutoName() {
  isNameManuallyEdited.value = false
  updateAutoName()
}

// Fetch primary loader versions
async function loadPrimaryVersions(loader: PrimaryLoaderId) {
  if (loader === 'none') {
    primaryVersions.value = []
    selectedPrimaryVersion.value = ''
    return
  }
  loadingPrimaryVersions.value = true
  try {
    const list = await ipcClient.loadersVersions(
      loader as LoaderKind,
      props.mcVersion,
    )
    primaryVersions.value = list
    if (list.length > 0 && !selectedPrimaryVersion.value) {
      selectedPrimaryVersion.value = list[0].version
    }
  } catch {
    primaryVersions.value = []
  } finally {
    loadingPrimaryVersions.value = false
    updateAutoName()
  }
}

// Select a primary loader
async function selectPrimary(id: PrimaryLoaderId) {
  if (selectedPrimary.value === id) return
  selectedPrimary.value = id
  selectedPrimaryVersion.value = ''

  if (incomp.value.optifineDisabledReason) {
    optifineEnabled.value = false
  }
  if (incomp.value.liteloaderDisabledReason) {
    liteloaderEnabled.value = false
  }

  await loadPrimaryVersions(id)
  await fetchAddons()
}

// Fetch OptiFine versions
async function loadOptifineVersions() {
  if (!optifineEnabled.value) return
  loadingOptifineVersions.value = true
  try {
    const list = await ipcClient.loadersVersions('optiFine', props.mcVersion)
    optifineVersions.value = list
    if (list.length > 0 && !selectedOptifineVersion.value) {
      selectedOptifineVersion.value = list[0].version
    }
  } catch {
    optifineVersions.value = []
  } finally {
    loadingOptifineVersions.value = false
    updateAutoName()
  }
}

// Fetch LiteLoader versions
async function loadLiteloaderVersions() {
  if (!liteloaderEnabled.value) return
  loadingLiteloaderVersions.value = true
  try {
    const list = await ipcClient.loadersVersions('liteLoader', props.mcVersion)
    liteloaderVersions.value = list
    if (list.length > 0 && !selectedLiteloaderVersion.value) {
      selectedLiteloaderVersion.value = list[0].version
    }
  } catch {
    liteloaderVersions.value = []
  } finally {
    loadingLiteloaderVersions.value = false
    updateAutoName()
  }
}

// Fetch add-ons candidates
async function fetchAddons() {
  const isFabric = selectedPrimary.value === 'fabric'
  const isLegacyFabric = selectedPrimary.value === 'legacyFabric'
  const isOpti = optifineEnabled.value

  if (!isFabric && !isLegacyFabric && !isOpti) {
    fabricApiCandidates.value = []
    legacyFabricApiCandidates.value = []
    optiFabricCandidates.value = []
    return
  }

  loadingExtras.value = true
  try {
    const extras = await ipcClient.installExtras(
      props.mcVersion,
      isFabric,
      isLegacyFabric,
      isFabric && isOpti,
    )
    fabricApiCandidates.value = extras.fabricApi
    legacyFabricApiCandidates.value = extras.legacyFabricApi
    optiFabricCandidates.value = extras.optiFabric
  } catch {
    // Graceful fallback
  } finally {
    loadingExtras.value = false
  }
}

// Watchers for additional toggles
watch(optifineEnabled, async (val) => {
  if (val) {
    await loadOptifineVersions()
    await fetchAddons()
  } else {
    optiFabricCandidates.value = []
    updateAutoName()
  }
})

watch(liteloaderEnabled, async (val) => {
  if (val) {
    await loadLiteloaderVersions()
  } else {
    updateAutoName()
  }
})

// Summary string for diff in modify mode
const currentSelectionSummary = computed(() => {
  const parts: string[] = []
  if (selectedPrimary.value !== 'none') {
    parts.push(
      `${selectedPrimary.value} ${selectedPrimaryVersion.value || ''}`.trim(),
    )
  }
  if (optifineEnabled.value) {
    parts.push(`OptiFine ${selectedOptifineVersion.value || ''}`.trim())
  }
  if (liteloaderEnabled.value) {
    parts.push(`LiteLoader ${selectedLiteloaderVersion.value || ''}`.trim())
  }
  if (selectedPrimary.value === 'fabric' && fabricApiEnabled.value) {
    parts.push('Fabric API')
  }
  if (selectedPrimary.value === 'legacyFabric' && legacyFabricApiEnabled.value) {
    parts.push('Legacy Fabric API')
  }
  if (
    selectedPrimary.value === 'fabric' &&
    optifineEnabled.value &&
    optiFabricEnabled.value
  ) {
    parts.push('OptiFabric')
  }
  return parts.length > 0 ? parts.join(' + ') : t('download.configurator.vanillaSummary')
})

const isModified = computed(() => {
  return currentSelectionSummary.value !== initialSummary.value
})

onMounted(async () => {
  if (props.mode === 'modify' && props.instance) {
    try {
      const info = await ipcClient.installCurrentInfo(props.instance.id)
      installedModFiles.value = []
      if (info.extras.fabricApi)
        installedModFiles.value.push(info.extras.fabricApi)
      if (info.extras.legacyFabricApi)
        installedModFiles.value.push(info.extras.legacyFabricApi)
      if (info.extras.optifabric)
        installedModFiles.value.push(info.extras.optifabric)

      // Detect current primary loader
      for (const loaderStr of info.loaders) {
        const lower = loaderStr.toLowerCase()
        if (lower.includes('fabric') && !lower.includes('legacy')) {
          selectedPrimary.value = 'fabric'
          const parts = loaderStr.split(' ')
          if (parts.length > 1) selectedPrimaryVersion.value = parts[1]
        } else if (lower.includes('legacyfabric')) {
          selectedPrimary.value = 'legacyFabric'
          const parts = loaderStr.split(' ')
          if (parts.length > 1) selectedPrimaryVersion.value = parts[1]
        } else if (lower.includes('neoforge')) {
          selectedPrimary.value = 'neoforge'
          const parts = loaderStr.split(' ')
          if (parts.length > 1) selectedPrimaryVersion.value = parts[1]
        } else if (lower.includes('forge')) {
          selectedPrimary.value = 'forge'
          const parts = loaderStr.split(' ')
          if (parts.length > 1) selectedPrimaryVersion.value = parts[1]
        } else if (lower.includes('cleanroom')) {
          selectedPrimary.value = 'cleanroom'
          const parts = loaderStr.split(' ')
          if (parts.length > 1) selectedPrimaryVersion.value = parts[1]
        } else if (lower.includes('optifine')) {
          optifineEnabled.value = true
        } else if (lower.includes('liteloader')) {
          liteloaderEnabled.value = true
        }
      }

      await loadPrimaryVersions(selectedPrimary.value)
      if (optifineEnabled.value) await loadOptifineVersions()
      if (liteloaderEnabled.value) await loadLiteloaderVersions()
      await fetchAddons()

      fabricApiEnabled.value = Boolean(info.extras.fabricApi)
      legacyFabricApiEnabled.value = Boolean(info.extras.legacyFabricApi)
      optiFabricEnabled.value = Boolean(info.extras.optifabric)

      initialSummary.value = currentSelectionSummary.value
    } catch {
      initialSummary.value = 'Vanilla'
    }
  } else {
    updateAutoName()
  }
})

// Submission handler
async function handleInstall() {
  if (props.mode === 'create' && nameValidationError.value) {
    return
  }
  submitting.value = true

  try {
    const extraMods: ExtraMod[] = []

    if (
      selectedPrimary.value === 'fabric' &&
      fabricApiEnabled.value &&
      fabricApiCandidates.value.length > 0
    ) {
      const best = fabricApiCandidates.value[0]
      extraMods.push({
        fileName: best.fileName,
        urls: best.downloadUrls,
        sha1: best.hashes.sha1 ?? null,
        size: best.size,
      })
    }

    if (
      selectedPrimary.value === 'legacyFabric' &&
      legacyFabricApiEnabled.value &&
      legacyFabricApiCandidates.value.length > 0
    ) {
      const best = legacyFabricApiCandidates.value[0]
      extraMods.push({
        fileName: best.fileName,
        urls: best.downloadUrls,
        sha1: best.hashes.sha1 ?? null,
        size: best.size,
      })
    }

    if (
      selectedPrimary.value === 'fabric' &&
      optifineEnabled.value &&
      optiFabricEnabled.value &&
      optiFabricCandidates.value.length > 0
    ) {
      const best = optiFabricCandidates.value[0]
      extraMods.push({
        fileName: best.fileName,
        urls: best.downloadUrls,
        sha1: best.hashes.sha1 ?? null,
        size: best.size,
      })
    }

    const modePayload: InstallMode =
      props.mode === 'modify'
        ? {
            type: 'modify',
            removeModFiles: installedModFiles.value,
          }
        : { type: 'create' }

    const targetName =
      props.mode === 'modify' && props.instance
        ? props.instance.id
        : instanceName.value.trim()

    const req: InstallRequest = {
      mcVersion: props.mcVersion,
      instanceName: targetName,
      fabric:
        selectedPrimary.value === 'fabric'
          ? selectedPrimaryVersion.value || 'latest'
          : null,
      legacyFabric:
        selectedPrimary.value === 'legacyFabric'
          ? selectedPrimaryVersion.value || 'latest'
          : null,
      forge:
        selectedPrimary.value === 'forge'
          ? selectedPrimaryVersion.value || 'latest'
          : null,
      neoforge:
        selectedPrimary.value === 'neoforge'
          ? selectedPrimaryVersion.value || 'latest'
          : null,
      cleanroom:
        selectedPrimary.value === 'cleanroom'
          ? selectedPrimaryVersion.value || 'latest'
          : null,
      labymod: null,
      liteloader: liteloaderEnabled.value
        ? selectedLiteloaderVersion.value || 'latest'
        : null,
      optifine: optifineEnabled.value
        ? selectedOptifineVersion.value || 'latest'
        : null,
      extraMods,
      mode: modePayload,
    }

    const taskId = await ipcClient.installCreate(req)
    toast.success(
      props.mode === 'modify'
        ? t('download.modify.modifySuccess', { name: targetName })
        : t('download.configurator.installSuccess', { name: targetName }),
    )

    emit('success', taskId)
    router?.push?.('/tasks')
  } catch (err: unknown) {
    const msg =
      err instanceof Error ? err.message : t('download.dialog.nameError')
    toast.danger(msg)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="m-configurator flex flex-col gap-6">
    <!-- Quilt warning if in modify mode -->
    <div
      v-if="mode === 'modify' && isQuiltInstance"
      class="p-4 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-300 text-sm flex items-center gap-3"
    >
      <span class="i-lucide-alert-triangle text-xl shrink-0" />
      <span>{{ t('download.modify.quiltUnsupported') }}</span>
    </div>

    <!-- Step 2 Header with MC Version Banner -->
    <div
      class="flex items-center justify-between p-4 rounded-2xl bg-white/5 border border-white/10"
    >
      <div class="flex items-center gap-3">
        <span class="i-lucide-box text-2xl text-primary" />
        <div>
          <div class="text-xs text-fg-muted">
            {{ t('download.configurator.targetVersion') }}
          </div>
          <div class="text-lg font-bold flex items-center gap-2">
            {{ mcVersion }}
            <MBadge variant="primary" size="sm">
              Drop {{ dropOf(mcVersion) }}
            </MBadge>
          </div>
        </div>
      </div>
      <MButton
        v-if="mode === 'create'"
        variant="text"
        size="sm"
        @click="emit('back')"
      >
        <span class="i-lucide-refresh-cw mr-1.5" />
        {{ t('download.configurator.changeVersion') }}
      </MButton>
    </div>

    <!-- Section 1: Primary Loader -->
    <div class="flex flex-col gap-3">
      <div class="text-sm font-semibold text-fg-muted flex items-center gap-2">
        <span>{{ t('download.configurator.primaryLoader') }}</span>
      </div>
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <button
          v-for="loader in primaryLoaderDefs"
          v-show="loader.visible"
          :key="loader.id"
          type="button"
          class="flex items-center gap-3 p-3.5 rounded-xl border transition-all text-left group cursor-pointer"
          :class="[
            selectedPrimary === loader.id
              ? 'border-primary bg-primary/10 shadow-sm shadow-primary/20 ring-1 ring-primary'
              : 'border-white/10 bg-white/5 hover:border-white/20 hover:bg-white/10',
          ]"
          @click="selectPrimary(loader.id)"
        >
          <span
            :class="[loader.icon, 'text-2xl']"
            :style="{
              color:
                selectedPrimary === loader.id
                  ? 'var(--primary)'
                  : 'var(--fg-muted)',
            }"
          />
          <div class="flex flex-col">
            <span class="font-medium text-sm text-primary">{{
              loader.label
            }}</span>
            <span class="text-xs text-fg-muted" v-if="selectedPrimary === loader.id">{{ t('download.configurator.selected') }}</span>
          </div>
        </button>
      </div>

      <!-- Primary loader version select -->
      <div
        v-if="selectedPrimary !== 'none'"
        class="mt-2 p-4 rounded-xl border border-white/10 bg-white/5 flex flex-col gap-2"
      >
        <div class="text-xs text-fg-muted flex items-center justify-between">
          <span>{{ selectedPrimary }} {{ t('download.configurator.versionSelect') }}</span>
          <span
            v-if="loadingPrimaryVersions"
            class="text-xs text-fg-muted animate-pulse"
          >
            {{ t('download.configurator.fetchingVersions') }}
          </span>
        </div>
        <MSkeleton v-if="loadingPrimaryVersions" class="h-10 w-full rounded-lg" />
        <MSelect
          v-else
          v-model="selectedPrimaryVersion"
          :options="
            primaryVersions.map((v) => ({
              label: `${v.version} ${v.stable ? t('download.configurator.recommended') : t('download.configurator.testing')}`,
              value: v.version,
            }))
          "
          @update:model-value="updateAutoName"
        />
      </div>
    </div>

    <!-- Section 2: Additional Combinations (OptiFine, LiteLoader) -->
    <div
      v-if="visibility.optifine || visibility.liteloader"
      class="flex flex-col gap-3"
    >
      <div class="text-sm font-semibold text-fg-muted">
        {{ t('download.configurator.additional') }}
      </div>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <!-- OptiFine Card -->
        <MCard
          v-if="visibility.optifine"
          class="p-4 flex flex-col gap-3"
          :class="[
            incomp.optifineDisabledReason ? 'opacity-50 pointer-events-none' : '',
          ]"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <span class="i-lucide-sparkles text-xl text-amber-400" />
              <div>
                <div class="font-medium text-sm">{{ t('download.configurator.optifineTitle') }}</div>
                <div class="text-xs text-fg-muted">{{ t('download.configurator.optifineDesc') }}</div>
              </div>
            </div>
            <MSwitch
              v-model="optifineEnabled"
              :disabled="Boolean(incomp.optifineDisabledReason)"
            />
          </div>

          <div
            v-if="incomp.optifineDisabledReason"
            class="text-xs text-amber-400/90"
          >
            {{ t(incomp.optifineDisabledReason) }}
          </div>

          <!-- OptiFine Version Picker -->
          <div v-if="optifineEnabled" class="flex flex-col gap-1.5 pt-2">
            <MSkeleton
              v-if="loadingOptifineVersions"
              class="h-9 w-full rounded-lg"
            />
            <MSelect
              v-else
              v-model="selectedOptifineVersion"
              :options="
                optifineVersions.map((v) => ({
                  label: `OptiFine ${v.version}`,
                  value: v.version,
                }))
              "
              @update:model-value="updateAutoName"
            />
          </div>
        </MCard>

        <!-- LiteLoader Card -->
        <MCard
          v-if="visibility.liteloader"
          class="p-4 flex flex-col gap-3"
          :class="[
            incomp.liteloaderDisabledReason
              ? 'opacity-50 pointer-events-none'
              : '',
          ]"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <span class="i-lucide-feather text-xl text-cyan-400" />
              <div>
                <div class="font-medium text-sm">{{ t('download.configurator.liteloaderTitle') }}</div>
                <div class="text-xs text-fg-muted">{{ t('download.configurator.liteloaderDesc') }}</div>
              </div>
            </div>
            <MSwitch
              v-model="liteloaderEnabled"
              :disabled="Boolean(incomp.liteloaderDisabledReason)"
            />
          </div>

          <div
            v-if="incomp.liteloaderDisabledReason"
            class="text-xs text-amber-400/90"
          >
            {{ t(incomp.liteloaderDisabledReason) }}
          </div>

          <!-- LiteLoader Version Picker -->
          <div v-if="liteloaderEnabled" class="flex flex-col gap-1.5 pt-2">
            <MSkeleton
              v-if="loadingLiteloaderVersions"
              class="h-9 w-full rounded-lg"
            />
            <MSelect
              v-else
              v-model="selectedLiteloaderVersion"
              :options="
                liteloaderVersions.map((v) => ({
                  label: `LiteLoader ${v.version}`,
                  value: v.version,
                }))
              "
              @update:model-value="updateAutoName"
            />
          </div>
        </MCard>
      </div>
    </div>

    <!-- Section 3: Add-on cards (Fabric API, Legacy Fabric API, OptiFabric) -->
    <div
      v-if="
        (selectedPrimary === 'fabric' && fabricApiCandidates.length > 0) ||
        (selectedPrimary === 'legacyFabric' &&
          legacyFabricApiCandidates.length > 0) ||
        (selectedPrimary === 'fabric' &&
          optifineEnabled &&
          optiFabricCandidates.length > 0)
      "
      class="flex flex-col gap-3"
    >
      <div class="text-sm font-semibold text-fg-muted">
        {{ t('download.addons.title') }}
      </div>

      <!-- Fabric API Card -->
      <MCard
        v-if="selectedPrimary === 'fabric' && fabricApiCandidates.length > 0"
        class="p-3.5 flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-puzzle text-xl text-primary" />
          <div>
            <div class="text-sm font-medium flex items-center gap-2">
              {{ t('download.addons.fabricApi') }}
              <MBadge size="sm" variant="success">{{ t('download.configurator.matchedBest') }}</MBadge>
            </div>
            <div class="text-xs text-fg-muted">
              {{ fabricApiCandidates[0]?.fileName }}
            </div>
          </div>
        </div>
        <MSwitch v-model="fabricApiEnabled" />
      </MCard>

      <!-- Legacy Fabric API Card -->
      <MCard
        v-if="
          selectedPrimary === 'legacyFabric' &&
          legacyFabricApiCandidates.length > 0
        "
        class="p-3.5 flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-puzzle text-xl text-primary" />
          <div>
            <div class="text-sm font-medium flex items-center gap-2">
              {{ t('download.addons.legacyFabricApi') }}
              <MBadge size="sm" variant="success">{{ t('download.configurator.matchedBest') }}</MBadge>
            </div>
            <div class="text-xs text-fg-muted">
              {{ legacyFabricApiCandidates[0]?.fileName }}
            </div>
          </div>
        </div>
        <MSwitch v-model="legacyFabricApiEnabled" />
      </MCard>

      <!-- OptiFabric Card -->
      <MCard
        v-if="
          selectedPrimary === 'fabric' &&
          optifineEnabled &&
          optiFabricCandidates.length > 0
        "
        class="p-3.5 flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-sparkle text-xl text-amber-400" />
          <div>
            <div class="text-sm font-medium flex items-center gap-2">
              {{ t('download.addons.optifabric') }}
              <MBadge size="sm" variant="success">{{ t('download.configurator.matchedBest') }}</MBadge>
            </div>
            <div class="text-xs text-fg-muted">
              {{ optiFabricCandidates[0]?.fileName }}
            </div>
          </div>
        </div>
        <MSwitch v-model="optiFabricEnabled" />
      </MCard>
    </div>

    <!-- Section 4: Hint Banners -->
    <div class="flex flex-col gap-2">
      <!-- Missing Fabric API warning (Red) -->
      <div
        v-if="selectedPrimary === 'fabric' && !fabricApiEnabled"
        class="p-3.5 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-alert-circle text-lg shrink-0" />
        <span>{{ t('download.hints.fabricApiMissing') }}</span>
      </div>

      <!-- Missing Legacy Fabric API warning (Red) -->
      <div
        v-if="selectedPrimary === 'legacyFabric' && !legacyFabricApiEnabled"
        class="p-3.5 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-alert-circle text-lg shrink-0" />
        <span>{{ t('download.hints.legacyFabricApiMissing') }}</span>
      </div>

      <!-- Missing OptiFabric warning (Red) -->
      <div
        v-if="
          selectedPrimary === 'fabric' && optifineEnabled && !optiFabricEnabled
        "
        class="p-3.5 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-alert-circle text-lg shrink-0" />
        <span>{{ t('download.hints.optifabricMissing') }}</span>
      </div>

      <!-- OptiFine + Mod coexistence warning on drop >= 160 (Yellow) -->
      <div
        v-if="
          optifineEnabled &&
          selectedPrimary !== 'none' &&
          dropOf(mcVersion) >= 160
        "
        class="p-3.5 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-info text-lg shrink-0" />
        <span>{{ t('download.hints.optifineModWarning') }}</span>
      </div>
    </div>

    <!-- Section 5: Instance Name Input (Create mode only) -->
    <div v-if="mode === 'create'" class="flex flex-col gap-2">
      <div class="flex items-center justify-between text-sm">
        <label class="font-medium text-fg-muted">
          {{ t('download.configurator.instanceName') }}
        </label>
        <div class="flex items-center gap-2">
          <MBadge v-if="!isNameManuallyEdited" size="sm" variant="neutral">
            {{ t('download.configurator.autoNamed') }}
          </MBadge>
          <MButton
            v-else
            variant="text"
            size="xs"
            @click="resetToAutoName"
          >
            {{ t('download.configurator.autoNamed') }}
          </MButton>
        </div>
      </div>
      <MInput
        :model-value="instanceName"
        :placeholder="t('download.configurator.namePlaceholder')"
        @update:model-value="handleNameInput"
      />
      <div v-if="nameValidationError" class="text-xs text-red-400">
        {{ t(nameValidationError) }}
      </div>
    </div>

    <!-- Section 6: Diff in Modify mode -->
    <div
      v-if="mode === 'modify' && !isQuiltInstance"
      class="p-4 rounded-xl border border-white/10 bg-white/5 flex flex-col gap-1.5"
    >
      <div class="text-xs text-fg-muted">
        {{ t('download.modify.diffLabel') }}
      </div>
      <div class="text-sm font-medium flex items-center gap-2">
        <span>{{ initialSummary || 'Vanilla' }}</span>
        <span class="i-lucide-arrow-right text-xs text-fg-muted" />
        <span class="text-primary">{{ currentSelectionSummary }}</span>
      </div>
    </div>

    <!-- Sticky Bottom Action Bar -->
    <div
      class="sticky bottom-0 -mx-4 -mb-4 p-4 bg-raised/80 backdrop-blur-md border-t border-white/10 flex items-center justify-between rounded-b-2xl z-20 m-slide-up"
    >
      <div class="flex flex-col">
        <div class="text-xs text-fg-muted">{{ t('download.configurator.summaryLabel') }}</div>
        <div class="text-sm font-semibold text-primary truncate max-w-xs sm:max-w-md">
          {{ currentSelectionSummary }}
        </div>
      </div>

      <MButton
        variant="primary"
        size="md"
        :loading="submitting"
        :disabled="
          (mode === 'create' && Boolean(nameValidationError)) ||
          (mode === 'modify' && isQuiltInstance) ||
          loadingPrimaryVersions
        "
        @click="handleInstall"
      >
        <span
          :class="[
            mode === 'modify' ? 'i-lucide-wrench' : 'i-lucide-download',
            'mr-2',
          ]"
        />
        {{
          mode === 'modify'
            ? isModified
              ? t('download.configurator.modifyAction')
              : t('download.configurator.resetAction')
            : t('download.configurator.installAction')
        }}
      </MButton>
    </div>
  </div>
</template>

<style scoped>
.m-configurator {
  position: relative;
}
</style>

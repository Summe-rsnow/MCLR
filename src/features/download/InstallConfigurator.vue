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
  type VersionCompatInfo,
} from '../../ipc/client'
import { useToast } from '../../ui'
import { useInstancesStore } from '../instances/store'
import {
  defaultName,
  dropOf,
  loaderDisplayName,
  loaderIncompatibilities,
  loaderVisibility,
  validateInstanceNameLocal,
} from './compat'
import ConfiguratorVersionStep from './components/ConfiguratorVersionStep.vue'
import ConfiguratorLoaderStep, {
  type PrimaryLoaderId,
  type LoaderDef,
} from './components/ConfiguratorLoaderStep.vue'
import ConfiguratorInstanceStep from './components/ConfiguratorInstanceStep.vue'

const instancesStore = useInstancesStore()

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

// Version compat from backend
const versionCompat = ref<VersionCompatInfo | null>(null)

// Incompatibilities & Visibility
const visibility = computed(() => {
  if (versionCompat.value) {
    const supported = versionCompat.value.supportedLoaders
    return {
      fabric: supported.includes('fabric'),
      forge: supported.includes('forge'),
      neoforge: supported.includes('neoforge'),
      cleanroom: supported.includes('cleanroom'),
      legacyFabric: supported.includes('legacyfabric'),
      liteloader: supported.includes('liteloader'),
      labymod: supported.includes('labymod'),
      optifine: supported.includes('optifine'),
    }
  }
  return loaderVisibility(props.mcVersion)
})

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
  return props.instance?.loaders?.some((l) => l.toLowerCase().includes('quilt')) ?? false
})

const primaryLoaderDefs = computed<LoaderDef[]>(() => [
  {
    id: 'none',
    label: t('download.configurator.vanillaPure'),
    visible: true,
    kind: 'vanilla',
  },
  {
    id: 'fabric',
    label: 'Fabric',
    visible: visibility.value.fabric,
    kind: 'fabric',
  },
  {
    id: 'neoforge',
    label: 'NeoForge',
    visible: visibility.value.neoforge,
    kind: 'neoforge',
  },
  {
    id: 'forge',
    label: 'Forge',
    visible: visibility.value.forge,
    kind: 'forge',
  },
  {
    id: 'cleanroom',
    label: 'Cleanroom',
    visible: visibility.value.cleanroom,
    kind: 'cleanroom',
  },
  {
    id: 'legacyFabric',
    label: 'LegacyFabric',
    visible: visibility.value.legacyFabric,
    kind: 'legacyfabric',
  },
  {
    id: 'labymod',
    label: 'LabyMod',
    visible: visibility.value.labymod,
    kind: 'labymod',
  },
])

/** LabyMod only publishes a production channel today; unstable builds would be snapshots. */
const selectedLabyModChannel = computed<string | undefined>(() => {
  if (selectedPrimary.value !== 'labymod') return undefined
  const entry = primaryVersions.value.find((v) => v.version === selectedPrimaryVersion.value)
  if (!entry) return 'production'
  return entry.stable ? 'production' : 'snapshot'
})

/** Names already used in the target game folder, for instant duplicate feedback. */
const existingNames = computed(() =>
  instancesStore.instances.map((i) => i.name).filter((n): n is string => Boolean(n)),
)

function updateAutoName() {
  if (props.mode === 'modify' || isNameManuallyEdited.value) return
  const name = defaultName({
    mcVersion: props.mcVersion,
    fabricVersion: selectedPrimary.value === 'fabric' ? selectedPrimaryVersion.value : undefined,
    legacyFabricVersion:
      selectedPrimary.value === 'legacyFabric' ? selectedPrimaryVersion.value : undefined,
    forgeVersion: selectedPrimary.value === 'forge' ? selectedPrimaryVersion.value : undefined,
    neoforgeVersion:
      selectedPrimary.value === 'neoforge' ? selectedPrimaryVersion.value : undefined,
    cleanroomVersion:
      selectedPrimary.value === 'cleanroom' ? selectedPrimaryVersion.value : undefined,
    labymodVersion: selectedPrimary.value === 'labymod' ? selectedPrimaryVersion.value : undefined,
    labymodChannel: selectedPrimary.value === 'labymod' ? selectedLabyModChannel.value : undefined,
    liteloader: liteloaderEnabled.value,
    optifineDisplay: optifineEnabled.value ? selectedOptifineVersion.value : undefined,
  })
  instanceName.value = name
  validateName()
}

function validateName() {
  if (props.mode === 'modify') {
    nameValidationError.value = null
    return
  }
  const localErr = validateInstanceNameLocal(instanceName.value, existingNames.value)
  if (localErr) {
    nameValidationError.value = localErr
  } else {
    nameValidationError.value = null
    ipcClient.installValidateName(instanceName.value).catch((err: unknown) => {
      if (err && typeof err === 'object' && 'code' in err) {
        nameValidationError.value = String(err.code)
      }
    })
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

function toLoaderKind(loader: PrimaryLoaderId): LoaderKind {
  switch (loader) {
    case 'neoforge':
      return 'neoForge'
    case 'labymod':
      return 'labyMod'
    case 'legacyFabric':
      return 'legacyFabric'
    default:
      return loader as LoaderKind
  }
}

async function loadPrimaryVersions(loader: PrimaryLoaderId) {
  if (loader === 'none') {
    primaryVersions.value = []
    selectedPrimaryVersion.value = ''
    return
  }
  loadingPrimaryVersions.value = true
  try {
    const list = await ipcClient.loadersVersions(toLoaderKind(loader), props.mcVersion)
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
  // Switching loaders (including back to vanilla) must re-derive the name: only
  // version-picker changes used to trigger `updateAutoName`, so the previous
  // loader suffix stuck around after returning to the vanilla option.
  updateAutoName()
}

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

const currentSelectionSummary = computed(() => {
  const parts: string[] = []
  if (selectedPrimary.value !== 'none') {
    parts.push(
      `${loaderDisplayName(selectedPrimary.value)} ${selectedPrimaryVersion.value || ''}`.trim(),
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
  if (selectedPrimary.value === 'fabric' && optifineEnabled.value && optiFabricEnabled.value) {
    parts.push('OptiFabric')
  }
  return parts.length > 0 ? parts.join(' + ') : t('download.configurator.vanillaSummary')
})

const isModified = computed(() => {
  return currentSelectionSummary.value !== initialSummary.value
})

onMounted(async () => {
  // Instance names feed both the duplicate check and auto-name dedupe.
  void instancesStore.fetchInstances(false)
  ipcClient
    .installVersionCompat(props.mcVersion, 'release', '')
    .then((compat) => {
      versionCompat.value = compat
    })
    .catch(() => {})

  if (props.mode === 'modify' && props.instance) {
    try {
      const info = await ipcClient.installCurrentInfo(props.instance.id)
      installedModFiles.value = []
      if (info.extras.fabricApi) installedModFiles.value.push(info.extras.fabricApi)
      if (info.extras.legacyFabricApi) installedModFiles.value.push(info.extras.legacyFabricApi)
      if (info.extras.optifabric) installedModFiles.value.push(info.extras.optifabric)

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
            remove_mod_files: installedModFiles.value,
          }
        : { type: 'create' }

    const targetName =
      props.mode === 'modify' && props.instance ? props.instance.id : instanceName.value.trim()

    const req: InstallRequest = {
      mcVersion: props.mcVersion,
      instanceName: targetName,
      fabric: selectedPrimary.value === 'fabric' ? selectedPrimaryVersion.value || 'latest' : null,
      legacyFabric:
        selectedPrimary.value === 'legacyFabric' ? selectedPrimaryVersion.value || 'latest' : null,
      forge: selectedPrimary.value === 'forge' ? selectedPrimaryVersion.value || 'latest' : null,
      neoforge:
        selectedPrimary.value === 'neoforge' ? selectedPrimaryVersion.value || 'latest' : null,
      cleanroom:
        selectedPrimary.value === 'cleanroom' ? selectedPrimaryVersion.value || 'latest' : null,
      labymod:
        selectedPrimary.value === 'labymod'
          ? {
              channel: selectedLabyModChannel.value ?? 'production',
              commitRef: selectedPrimaryVersion.value || 'latest',
              baseVersion: selectedPrimaryVersion.value || 'latest',
            }
          : null,
      liteloader: liteloaderEnabled.value ? selectedLiteloaderVersion.value || 'latest' : null,
      optifine: optifineEnabled.value ? selectedOptifineVersion.value || 'latest' : null,
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
    const msg = err instanceof Error ? err.message : t('download.dialog.nameError')
    toast.danger(msg)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="m-configurator flex flex-col gap-6 min-h-[560px]">
    <div
      v-if="mode === 'modify' && isQuiltInstance"
      class="p-4 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-300 text-sm flex items-center gap-3"
    >
      <span class="i-lucide-triangle-alert text-xl shrink-0" />
      <span>{{ t('download.modify.quiltUnsupported') }}</span>
    </div>

    <!-- Step 2 Header with MC Version Banner -->
    <ConfiguratorVersionStep :mc-version="mcVersion" :mode="mode" @back="emit('back')" />

    <!-- Loaders & Addons Section -->
    <ConfiguratorLoaderStep
      v-model:selected-primary="selectedPrimary"
      v-model:selected-primary-version="selectedPrimaryVersion"
      v-model:optifine-enabled="optifineEnabled"
      v-model:selected-optifine-version="selectedOptifineVersion"
      v-model:liteloader-enabled="liteloaderEnabled"
      v-model:selected-liteloader-version="selectedLiteloaderVersion"
      v-model:fabric-api-enabled="fabricApiEnabled"
      v-model:legacy-fabric-api-enabled="legacyFabricApiEnabled"
      v-model:opti-fabric-enabled="optiFabricEnabled"
      :primary-loader-defs="primaryLoaderDefs"
      :visibility="visibility"
      :incomp="incomp"
      :primary-versions="primaryVersions"
      :loading-primary-versions="loadingPrimaryVersions"
      :optifine-versions="optifineVersions"
      :loading-optifine-versions="loadingOptifineVersions"
      :liteloader-versions="liteloaderVersions"
      :loading-liteloader-versions="loadingLiteloaderVersions"
      :fabric-api-candidates="fabricApiCandidates"
      :legacy-fabric-api-candidates="legacyFabricApiCandidates"
      :opti-fabric-candidates="optiFabricCandidates"
      :drop="dropOf(mcVersion)"
      @select-primary="selectPrimary"
      @update-auto-name="updateAutoName"
    />

    <!-- Instance Name & Action Step -->
    <ConfiguratorInstanceStep
      :mode="mode"
      :instance-name="instanceName"
      :is-name-manually-edited="isNameManuallyEdited"
      :name-validation-error="nameValidationError"
      :is-quilt-instance="isQuiltInstance"
      :initial-summary="initialSummary"
      :current-selection-summary="currentSelectionSummary"
      :submitting="submitting"
      :is-modified="isModified"
      :loading-primary-versions="loadingPrimaryVersions"
      @input-name="handleNameInput"
      @reset-auto-name="resetToAutoName"
      @install="handleInstall"
    />
  </div>
</template>

<style scoped>
.m-configurator {
  position: relative;
}
</style>

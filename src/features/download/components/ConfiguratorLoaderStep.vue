<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CompFile, LoaderVersionEntry } from '../../../ipc/client'
import { MCard, MSelect, MBadge, MSwitch, MSkeleton, MLoaderIcon } from '../../../ui'

export type PrimaryLoaderId =
  | 'none'
  | 'fabric'
  | 'neoforge'
  | 'forge'
  | 'cleanroom'
  | 'legacyFabric'
  | 'labymod'

export interface LoaderDef {
  id: PrimaryLoaderId
  label: string
  visible: boolean
  /** Loader kind understood by `MLoaderIcon` / `getLoaderMeta` (official artwork). */
  kind: string
}

export interface IncompInfo {
  optifineDisabledReason?: string | null
  liteloaderDisabledReason?: string | null
}

export interface VisibilityInfo {
  fabric: boolean
  forge: boolean
  neoforge: boolean
  cleanroom: boolean
  legacyFabric: boolean
  liteloader: boolean
  labymod: boolean
  optifine: boolean
}

const props = defineProps<{
  primaryLoaderDefs: LoaderDef[]
  visibility: VisibilityInfo
  incomp: IncompInfo
  primaryVersions: LoaderVersionEntry[]
  loadingPrimaryVersions: boolean
  optifineVersions: LoaderVersionEntry[]
  loadingOptifineVersions: boolean
  liteloaderVersions: LoaderVersionEntry[]
  loadingLiteloaderVersions: boolean
  fabricApiCandidates: CompFile[]
  legacyFabricApiCandidates: CompFile[]
  optiFabricCandidates: CompFile[]
  drop: number
}>()

const selectedPrimary = defineModel<PrimaryLoaderId>('selectedPrimary', { required: true })
/** Human-readable label of the selected loader ("LabyMod", not the raw `labymod` id). */
const selectedLoaderLabel = computed(
  () =>
    props.primaryLoaderDefs.find((d) => d.id === selectedPrimary.value)?.label ??
    String(selectedPrimary.value),
)
const selectedPrimaryVersion = defineModel<string>('selectedPrimaryVersion', { required: true })
const optifineEnabled = defineModel<boolean>('optifineEnabled', { required: true })
const selectedOptifineVersion = defineModel<string>('selectedOptifineVersion', { required: true })
const liteloaderEnabled = defineModel<boolean>('liteloaderEnabled', { required: true })
const selectedLiteloaderVersion = defineModel<string>('selectedLiteloaderVersion', {
  required: true,
})
const fabricApiEnabled = defineModel<boolean>('fabricApiEnabled', { required: true })
const legacyFabricApiEnabled = defineModel<boolean>('legacyFabricApiEnabled', { required: true })
const optiFabricEnabled = defineModel<boolean>('optiFabricEnabled', { required: true })

const emit = defineEmits<{
  (e: 'select-primary', id: PrimaryLoaderId): void
  (e: 'update-auto-name'): void
}>()

const { t } = useI18n()

/**
 * Incompatible extras are hidden instead of shown disabled: a card the user can
 * never enable is dead weight (it also kept reserving a whole row).
 */
const showOptifineCard = computed(
  () => props.visibility.optifine && !props.incomp.optifineDisabledReason,
)
const showLiteloaderCard = computed(
  () => props.visibility.liteloader && !props.incomp.liteloaderDisabledReason,
)
</script>

<template>
  <div class="flex flex-col gap-4">
    <!-- Section 1: Primary Loader -->
    <div class="flex flex-col gap-2.5">
      <div class="text-sm font-semibold text-fg-muted flex items-center gap-2">
        <span>{{ t('download.configurator.primaryLoader') }}</span>
      </div>
      <div
        class="m-loader-grid grid gap-2 p-1 grid-cols-[repeat(auto-fit,minmax(150px,1fr))]"
        data-testid="primary-loader-grid"
      >
        <button
          v-for="loader in primaryLoaderDefs"
          v-show="loader.visible"
          :key="loader.id"
          type="button"
          data-testid="primary-loader-card"
          class="flex items-center gap-2 py-1.5 pl-1.5 pr-3 rounded-xl border transition-all text-left group cursor-pointer min-w-0"
          :class="[
            selectedPrimary === loader.id
              ? 'border-primary bg-primary/10 shadow-sm shadow-primary/20 ring-1 ring-primary'
              : 'border-white/10 bg-white/5 hover:border-white/20 hover:bg-white/10',
          ]"
          @click="emit('select-primary', loader.id)"
        >
          <MLoaderIcon :kind="loader.kind" :size="24" />
          <div class="flex flex-col min-w-0">
            <span
              class="font-medium text-sm truncate"
              :class="selectedPrimary === loader.id ? 'text-primary' : 'text-fg'"
              >{{ loader.label }}</span
            >
            <span v-if="selectedPrimary === loader.id" class="text-xs text-fg-muted truncate">{{
              t('download.configurator.selected')
            }}</span>
          </div>
        </button>
      </div>

      <!-- Primary loader version select (compact, next to the loader label) -->
      <div v-if="selectedPrimary !== 'none'" class="mt-1 flex flex-wrap items-center gap-3">
        <span class="text-xs text-fg-muted shrink-0">
          {{ selectedLoaderLabel }} {{ t('download.configurator.versionSelect') }}
        </span>
        <div class="w-64 max-w-full">
          <MSkeleton v-if="loadingPrimaryVersions" class="h-9 w-full rounded-lg" />
          <MSelect
            v-else
            v-model="selectedPrimaryVersion"
            :options="
              primaryVersions.map((v) => ({
                label: `${v.version}${v.stable ? ' ' + t('download.configurator.recommended') : ''}`,
                value: v.version,
              }))
            "
            size="sm"
            @update:model-value="emit('update-auto-name')"
          />
        </div>
        <span v-if="loadingPrimaryVersions" class="text-xs text-fg-muted animate-pulse">
          {{ t('download.configurator.fetchingVersions') }}
        </span>
      </div>
    </div>

    <!-- Section 2: Additional Combinations — incompatible extras are hidden, not disabled -->
    <div v-if="showOptifineCard || showLiteloaderCard" class="flex flex-col gap-3">
      <div class="text-sm font-semibold text-fg-muted">
        {{ t('download.configurator.additional') }}
      </div>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
        <!-- OptiFine Card -->
        <MCard v-if="showOptifineCard" padding="none" class="p-3 flex flex-col gap-2.5">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <MLoaderIcon kind="optifine" :size="24" />
              <div>
                <div class="font-medium text-sm">
                  {{ t('download.configurator.optifineTitle') }}
                </div>
                <div class="text-xs text-fg-muted">
                  {{ t('download.configurator.optifineDesc') }}
                </div>
              </div>
            </div>
            <MSwitch v-model="optifineEnabled" />
          </div>

          <!-- OptiFine Version Picker -->
          <div v-if="optifineEnabled" class="flex flex-col gap-1.5 pt-1">
            <MSkeleton v-if="loadingOptifineVersions" class="h-9 w-full rounded-lg" />
            <MSelect
              v-else
              v-model="selectedOptifineVersion"
              :options="
                optifineVersions.map((v) => ({
                  label: `OptiFine ${v.version}`,
                  value: v.version,
                }))
              "
              @update:model-value="emit('update-auto-name')"
            />
          </div>
        </MCard>

        <!-- LiteLoader Card -->
        <MCard v-if="showLiteloaderCard" padding="none" class="p-3 flex flex-col gap-2.5">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <MLoaderIcon kind="liteloader" :size="24" />
              <div>
                <div class="font-medium text-sm">
                  {{ t('download.configurator.liteloaderTitle') }}
                </div>
                <div class="text-xs text-fg-muted">
                  {{ t('download.configurator.liteloaderDesc') }}
                </div>
              </div>
            </div>
            <MSwitch v-model="liteloaderEnabled" />
          </div>

          <!-- LiteLoader Version Picker -->
          <div v-if="liteloaderEnabled" class="flex flex-col gap-1.5 pt-1">
            <MSkeleton v-if="loadingLiteloaderVersions" class="h-9 w-full rounded-lg" />
            <MSelect
              v-else
              v-model="selectedLiteloaderVersion"
              :options="
                liteloaderVersions.map((v) => ({
                  label: `LiteLoader ${v.version}`,
                  value: v.version,
                }))
              "
              @update:model-value="emit('update-auto-name')"
            />
          </div>
        </MCard>
      </div>
    </div>

    <!-- Section 3: Add-on cards (Fabric API, Legacy Fabric API, OptiFabric) -->
    <div
      v-if="
        (selectedPrimary === 'fabric' && fabricApiCandidates.length > 0) ||
        (selectedPrimary === 'legacyFabric' && legacyFabricApiCandidates.length > 0) ||
        (selectedPrimary === 'fabric' && optifineEnabled && optiFabricCandidates.length > 0)
      "
      class="flex flex-col gap-3"
    >
      <div class="text-sm font-semibold text-fg-muted">
        {{ t('download.addons.title') }}
      </div>

      <!-- Fabric API Card -->
      <MCard
        v-if="selectedPrimary === 'fabric' && fabricApiCandidates.length > 0"
        padding="none"
        class="p-3 flex items-center justify-between"
      >
        <div class="flex items-center gap-2.5">
          <span class="i-lucide-puzzle text-lg text-primary" />
          <div>
            <div class="text-sm font-medium flex items-center gap-2">
              {{ t('download.addons.fabricApi') }}
              <MBadge size="sm" variant="success">{{
                t('download.configurator.matchedBest')
              }}</MBadge>
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
        v-if="selectedPrimary === 'legacyFabric' && legacyFabricApiCandidates.length > 0"
        padding="none"
        class="p-3 flex items-center justify-between"
      >
        <div class="flex items-center gap-2.5">
          <span class="i-lucide-puzzle text-lg text-emerald-400" />
          <div>
            <div class="text-sm font-medium flex items-center gap-2">
              {{ t('download.addons.legacyFabricApi') }}
              <MBadge size="sm" variant="success">{{
                t('download.configurator.matchedBest')
              }}</MBadge>
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
        v-if="selectedPrimary === 'fabric' && optifineEnabled && optiFabricCandidates.length > 0"
        padding="none"
        class="p-3 flex items-center justify-between"
      >
        <div class="flex items-center gap-2.5">
          <span class="i-lucide-link text-lg text-cyan-400" />
          <div>
            <div class="text-sm font-medium flex items-center gap-2">
              {{ t('download.addons.optifabric') }}
              <MBadge size="sm" variant="success">{{
                t('download.configurator.matchedBest')
              }}</MBadge>
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
        class="p-3 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-circle-alert text-base shrink-0" />
        <span>{{ t('download.hints.fabricApiMissing') }}</span>
      </div>

      <!-- Missing Legacy Fabric API warning (Red) -->
      <div
        v-if="selectedPrimary === 'legacyFabric' && !legacyFabricApiEnabled"
        class="p-3 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-circle-alert text-base shrink-0" />
        <span>{{ t('download.hints.legacyFabricApiMissing') }}</span>
      </div>

      <!-- Missing OptiFabric warning (Red) -->
      <div
        v-if="selectedPrimary === 'fabric' && optifineEnabled && !optiFabricEnabled"
        class="p-3 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-circle-alert text-base shrink-0" />
        <span>{{ t('download.hints.optifabricMissing') }}</span>
      </div>

      <!-- OptiFine + Mod coexistence warning on drop >= 160 (Yellow) -->
      <div
        v-if="optifineEnabled && selectedPrimary !== 'none' && drop >= 160"
        class="p-3 rounded-xl border border-amber-500/30 bg-amber-500/10 text-amber-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-info text-base shrink-0" />
        <span>{{ t('download.hints.optifineModWarning') }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { CompFile, LoaderVersionEntry } from '../../../ipc/client'
import { MCard, MSelect, MBadge, MSwitch, MSkeleton } from '../../../ui'

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
  icon: string
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

defineProps<{
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
</script>

<template>
  <div class="flex flex-col gap-4">
    <!-- Section 1: Primary Loader -->
    <div class="flex flex-col gap-2.5">
      <div class="text-sm font-semibold text-fg-muted flex items-center gap-2">
        <span>{{ t('download.configurator.primaryLoader') }}</span>
      </div>
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-2.5">
        <button
          v-for="loader in primaryLoaderDefs"
          v-show="loader.visible"
          :key="loader.id"
          type="button"
          class="flex items-center gap-2.5 p-2.5 rounded-xl border transition-all text-left group cursor-pointer"
          :class="[
            selectedPrimary === loader.id
              ? 'border-primary bg-primary/10 shadow-sm shadow-primary/20 ring-1 ring-primary'
              : 'border-white/10 bg-white/5 hover:border-white/20 hover:bg-white/10',
          ]"
          @click="emit('select-primary', loader.id)"
        >
          <span
            :class="[loader.icon, 'text-xl']"
            :style="{
              color: selectedPrimary === loader.id ? 'var(--primary)' : 'var(--fg-muted)',
            }"
          />
          <div class="flex flex-col">
            <span class="font-medium text-sm text-primary">{{ loader.label }}</span>
            <span v-if="selectedPrimary === loader.id" class="text-xs text-fg-muted">{{
              t('download.configurator.selected')
            }}</span>
          </div>
        </button>
      </div>

      <!-- Primary loader version select -->
      <div
        v-if="selectedPrimary !== 'none'"
        class="mt-1 p-3 rounded-xl border border-white/10 bg-white/5 flex flex-col gap-2 min-h-[76px] justify-center"
      >
        <div class="text-xs text-fg-muted flex items-center justify-between">
          <span>{{ selectedPrimary }} {{ t('download.configurator.versionSelect') }}</span>
          <span v-if="loadingPrimaryVersions" class="text-xs text-fg-muted animate-pulse">
            {{ t('download.configurator.fetchingVersions') }}
          </span>
        </div>
        <MSkeleton v-if="loadingPrimaryVersions" class="h-10 w-full rounded-lg" />
        <MSelect
          v-else
          v-model="selectedPrimaryVersion"
          :options="
            primaryVersions.map((v) => ({
              label: `${v.version}${v.stable ? ' ' + t('download.configurator.recommended') : ''}`,
              value: v.version,
            }))
          "
          @update:model-value="emit('update-auto-name')"
        />
      </div>
    </div>

    <!-- Section 2: Additional Combinations (OptiFine, LiteLoader) -->
    <div v-if="visibility.optifine || visibility.liteloader" class="flex flex-col gap-3">
      <div class="text-sm font-semibold text-fg-muted">
        {{ t('download.configurator.additional') }}
      </div>
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5">
        <!-- OptiFine Card -->
        <MCard
          v-if="visibility.optifine"
          padding="none"
          class="p-3 flex flex-col gap-2.5"
          :class="[incomp.optifineDisabledReason ? 'opacity-50 pointer-events-none' : '']"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <span class="i-lucide-sparkles text-xl text-amber-400" />
              <div>
                <div class="font-medium text-sm">
                  {{ t('download.configurator.optifineTitle') }}
                </div>
                <div class="text-xs text-fg-muted">
                  {{ t('download.configurator.optifineDesc') }}
                </div>
              </div>
            </div>
            <MSwitch v-model="optifineEnabled" :disabled="Boolean(incomp.optifineDisabledReason)" />
          </div>

          <div v-if="incomp.optifineDisabledReason" class="text-xs text-amber-400/90">
            {{ t(incomp.optifineDisabledReason) }}
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
        <MCard
          v-if="visibility.liteloader"
          padding="none"
          class="p-3 flex flex-col gap-2.5"
          :class="[incomp.liteloaderDisabledReason ? 'opacity-50 pointer-events-none' : '']"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <span class="i-lucide-feather text-xl text-cyan-400" />
              <div>
                <div class="font-medium text-sm">
                  {{ t('download.configurator.liteloaderTitle') }}
                </div>
                <div class="text-xs text-fg-muted">
                  {{ t('download.configurator.liteloaderDesc') }}
                </div>
              </div>
            </div>
            <MSwitch
              v-model="liteloaderEnabled"
              :disabled="Boolean(incomp.liteloaderDisabledReason)"
            />
          </div>

          <div v-if="incomp.liteloaderDisabledReason" class="text-xs text-amber-400/90">
            {{ t(incomp.liteloaderDisabledReason) }}
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
        <span class="i-lucide-alert-circle text-base shrink-0" />
        <span>{{ t('download.hints.fabricApiMissing') }}</span>
      </div>

      <!-- Missing Legacy Fabric API warning (Red) -->
      <div
        v-if="selectedPrimary === 'legacyFabric' && !legacyFabricApiEnabled"
        class="p-3 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-alert-circle text-base shrink-0" />
        <span>{{ t('download.hints.legacyFabricApiMissing') }}</span>
      </div>

      <!-- Missing OptiFabric warning (Red) -->
      <div
        v-if="selectedPrimary === 'fabric' && optifineEnabled && !optiFabricEnabled"
        class="p-3 rounded-xl border border-red-500/30 bg-red-500/10 text-red-300 text-xs flex items-center gap-2.5"
      >
        <span class="i-lucide-alert-circle text-base shrink-0" />
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

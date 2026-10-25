<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { VersionManifestEntry } from '../../../ipc/client'
import { MCard, MBadge, MButton, MCollapse, MVirtualList } from '../../../ui'
import { getMcFoolName } from '../compat'

export interface LatestVersionItem extends VersionManifestEntry {
  category: 'release' | 'snapshot'
}

defineProps<{
  searchQuery: string
  latestVersions: LatestVersionItem[]
  releaseVersions: VersionManifestEntry[]
  snapshotVersions: VersionManifestEntry[]
  oldVersions: VersionManifestEntry[]
  aprilFoolsVersions: VersionManifestEntry[]
}>()

const openLatest = defineModel<boolean>('openLatest', { default: true })
const openRelease = defineModel<boolean>('openRelease', { default: true })
const openSnapshot = defineModel<boolean>('openSnapshot', { default: false })
const openOld = defineModel<boolean>('openOld', { default: false })
const openAprilFools = defineModel<boolean>('openAprilFools', { default: false })

const emit = defineEmits<{
  (e: 'select-version', id: string): void
}>()

const { t } = useI18n()

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString()
  } catch {
    return iso
  }
}
</script>

<template>
  <div class="flex flex-col gap-3">
    <!-- Latest Group -->
    <MCard
      v-if="!searchQuery && latestVersions.length > 0"
      padding="none"
      class="overflow-hidden transition-all"
    >
      <button
        type="button"
        class="w-full px-4 py-2.5 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors cursor-pointer"
        @click="openLatest = !openLatest"
      >
        <div class="flex items-center gap-3">
          <div
            class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
          >
            <span class="i-lucide-sparkles w-4 h-4" />
          </div>
          <span class="font-semibold text-sm text-fg">{{ t('download.groups.latest') }}</span>
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-base text-fg-muted transition-transform duration-200',
            openLatest ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openLatest">
        <div class="divide-y divide-line/40 border-t border-line/40">
          <div
            v-for="v in latestVersions"
            :key="v.id"
            class="px-4 py-2 flex items-center justify-between hover:bg-sunken/50 transition-colors cursor-pointer group"
            @click="emit('select-version', v.id)"
          >
            <div class="flex items-center gap-3">
              <div
                :class="[
                  'w-8 h-8 rounded-lg flex items-center justify-center shrink-0 border transition-all duration-200 group-hover:scale-105 shadow-sm',
                  v.category === 'release'
                    ? 'bg-emerald-500/15 text-emerald-500 border-emerald-500/30'
                    : 'bg-amber-500/15 text-amber-500 border-amber-500/30',
                ]"
              >
                <span
                  :class="[
                    'w-4 h-4',
                    v.category === 'release' ? 'i-lucide-box' : 'i-lucide-command',
                  ]"
                />
              </div>
              <div class="flex flex-col">
                <div class="flex items-center gap-2">
                  <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                    {{ v.id }}
                  </span>
                  <MBadge size="sm" :variant="v.category === 'release' ? 'primary' : 'neutral'">
                    {{
                      v.category === 'release'
                        ? t('download.groups.release')
                        : t('download.groups.snapshot')
                    }}
                  </MBadge>
                </div>
                <span class="text-xs text-fg-muted">
                  {{
                    v.category === 'release'
                      ? t('download.latestReleaseDesc', { date: formatDate(v.releaseTime) })
                      : t('download.latestSnapshotDesc', { date: formatDate(v.releaseTime) })
                  }}
                </span>
              </div>
            </div>
            <MButton variant="text" size="sm">
              <span class="i-lucide-arrow-right w-4 h-4" />
            </MButton>
          </div>
        </div>
      </MCollapse>
    </MCard>

    <!-- Release Group -->
    <MCard v-if="releaseVersions.length > 0" padding="none" class="overflow-hidden transition-all">
      <button
        type="button"
        class="w-full px-4 py-2.5 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors cursor-pointer"
        @click="openRelease = !openRelease"
      >
        <div class="flex items-center gap-3">
          <div
            class="w-8 h-8 rounded-lg bg-emerald-500/15 text-emerald-500 flex items-center justify-center shrink-0"
          >
            <span class="i-lucide-circle-check w-4 h-4" />
          </div>
          <span class="font-semibold text-sm text-fg"
            >{{ t('download.groups.release') }} ({{ releaseVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-base text-fg-muted transition-transform duration-200',
            openRelease ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openRelease">
        <div class="border-t border-line/40">
          <MVirtualList
            :items="releaseVersions"
            :item-key="(v) => v.id"
            :estimate-size="46"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-2 flex items-center justify-between hover:bg-sunken/50 transition-colors cursor-pointer group border-b border-line/30"
                @click="emit('select-version', v.id)"
              >
                <div class="flex items-center gap-3">
                  <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                    {{ v.id }}
                  </span>
                  <span class="text-xs text-fg-muted">
                    {{ formatDate(v.releaseTime) }}
                  </span>
                </div>
                <span
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors w-4 h-4"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>

    <!-- Snapshot Group -->
    <MCard v-if="snapshotVersions.length > 0" padding="none" class="overflow-hidden transition-all">
      <button
        type="button"
        class="w-full px-4 py-2.5 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors cursor-pointer"
        @click="openSnapshot = !openSnapshot"
      >
        <div class="flex items-center gap-3">
          <div
            class="w-8 h-8 rounded-lg bg-amber-500/15 text-amber-500 flex items-center justify-center shrink-0"
          >
            <span class="i-lucide-flask-conical w-4 h-4" />
          </div>
          <span class="font-semibold text-sm text-fg"
            >{{ t('download.groups.snapshot') }} ({{ snapshotVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-base text-fg-muted transition-transform duration-200',
            openSnapshot ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openSnapshot">
        <div class="border-t border-line/40">
          <MVirtualList
            :items="snapshotVersions"
            :item-key="(v) => v.id"
            :estimate-size="46"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-2 flex items-center justify-between hover:bg-sunken/50 transition-colors cursor-pointer group border-b border-line/30"
                @click="emit('select-version', v.id)"
              >
                <div class="flex items-center gap-3">
                  <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                    {{ v.id }}
                  </span>
                  <span class="text-xs text-fg-muted">
                    {{ formatDate(v.releaseTime) }}
                  </span>
                </div>
                <span
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors w-4 h-4"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>

    <!-- Old Group -->
    <MCard v-if="oldVersions.length > 0" padding="none" class="overflow-hidden transition-all">
      <button
        type="button"
        class="w-full px-4 py-2.5 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors cursor-pointer"
        @click="openOld = !openOld"
      >
        <div class="flex items-center gap-3">
          <div
            class="w-8 h-8 rounded-lg bg-stone-500/15 text-stone-400 flex items-center justify-center shrink-0"
          >
            <span class="i-lucide-archive w-4 h-4" />
          </div>
          <span class="font-semibold text-sm text-fg"
            >{{ t('download.groups.old') }} ({{ oldVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-base text-fg-muted transition-transform duration-200',
            openOld ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openOld">
        <div class="border-t border-line/40">
          <MVirtualList
            :items="oldVersions"
            :item-key="(v) => v.id"
            :estimate-size="46"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-2 flex items-center justify-between hover:bg-sunken/50 transition-colors cursor-pointer group border-b border-line/30"
                @click="emit('select-version', v.id)"
              >
                <div class="flex items-center gap-3">
                  <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                    {{ v.id }}
                  </span>
                  <MBadge size="sm" variant="neutral">
                    {{ v.versionType }}
                  </MBadge>
                  <span class="text-xs text-fg-muted">
                    {{ formatDate(v.releaseTime) }}
                  </span>
                </div>
                <span
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors w-4 h-4"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>

    <!-- April Fools Group -->
    <MCard
      v-if="aprilFoolsVersions.length > 0"
      padding="none"
      class="overflow-hidden transition-all"
    >
      <button
        type="button"
        class="w-full px-4 py-2.5 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors cursor-pointer"
        @click="openAprilFools = !openAprilFools"
      >
        <div class="flex items-center gap-3">
          <div
            class="w-8 h-8 rounded-lg bg-purple-500/15 text-purple-400 flex items-center justify-center shrink-0"
          >
            <span class="i-lucide-party-popper w-4 h-4" />
          </div>
          <span class="font-semibold text-sm text-fg"
            >{{ t('download.groups.aprilFools') }} ({{ aprilFoolsVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-base text-fg-muted transition-transform duration-200',
            openAprilFools ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openAprilFools">
        <div class="border-t border-line/40">
          <MVirtualList
            :items="aprilFoolsVersions"
            :item-key="(v) => v.id"
            :estimate-size="46"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-2 flex items-center justify-between hover:bg-sunken/50 transition-colors cursor-pointer group border-b border-line/30"
                @click="emit('select-version', v.id)"
              >
                <div class="flex items-center gap-3">
                  <span class="font-semibold text-sm group-hover:text-primary transition-colors">
                    {{ v.id }}
                  </span>
                  <span v-if="getMcFoolName(v.id)" class="text-xs text-purple-400 font-medium">
                    {{ getMcFoolName(v.id) }}
                  </span>
                  <span class="text-xs text-fg-muted">
                    {{ formatDate(v.releaseTime) }}
                  </span>
                </div>
                <span
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors w-4 h-4"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>
  </div>
</template>

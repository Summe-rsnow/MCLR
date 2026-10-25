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
  <div class="flex flex-col gap-4">
    <!-- Latest Group -->
    <MCard v-if="!searchQuery && latestVersions.length > 0" class="overflow-hidden">
      <button
        type="button"
        class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
        @click="openLatest = !openLatest"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-sparkles text-xl text-primary" />
          <span class="font-bold text-base">{{ t('download.groups.latest') }}</span>
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
            openLatest ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openLatest">
        <div class="divide-y divide-white/5 border-t border-white/5">
          <div
            v-for="v in latestVersions"
            :key="v.id"
            class="p-4 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group"
            @click="emit('select-version', v.id)"
          >
            <div class="flex items-center gap-3">
              <div
                :class="[
                  'w-10 h-10 rounded-lg flex items-center justify-center shrink-0 border transition-all duration-200 group-hover:scale-105 shadow-sm',
                  v.category === 'release'
                    ? 'bg-emerald-500/15 text-emerald-500 border-emerald-500/30'
                    : 'bg-amber-500/15 text-amber-500 border-amber-500/30',
                ]"
              >
                <span
                  :class="[
                    'text-xl',
                    v.category === 'release' ? 'i-lucide-box' : 'i-lucide-command',
                  ]"
                />
              </div>
              <div class="flex flex-col">
                <div class="flex items-center gap-2">
                  <span class="font-bold text-base group-hover:text-primary transition-colors">
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
                <span class="text-xs text-fg-muted mt-0.5">
                  {{
                    v.category === 'release'
                      ? t('download.latestReleaseDesc', { date: formatDate(v.releaseTime) })
                      : t('download.latestSnapshotDesc', { date: formatDate(v.releaseTime) })
                  }}
                </span>
              </div>
            </div>
            <MButton variant="text" size="sm">
              <span class="i-lucide-arrow-right" />
            </MButton>
          </div>
        </div>
      </MCollapse>
    </MCard>

    <!-- Release Group -->
    <MCard v-if="releaseVersions.length > 0" class="overflow-hidden">
      <button
        type="button"
        class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
        @click="openRelease = !openRelease"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-circle-check text-xl text-emerald-400" />
          <span class="font-bold text-base"
            >{{ t('download.groups.release') }} ({{ releaseVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
            openRelease ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openRelease">
        <div class="border-t border-white/5">
          <MVirtualList
            :items="releaseVersions"
            :item-key="(v) => v.id"
            :estimate-size="49"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
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
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>

    <!-- Snapshot Group -->
    <MCard v-if="snapshotVersions.length > 0" class="overflow-hidden">
      <button
        type="button"
        class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
        @click="openSnapshot = !openSnapshot"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-flask-conical text-xl text-amber-400" />
          <span class="font-bold text-base"
            >{{ t('download.groups.snapshot') }} ({{ snapshotVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
            openSnapshot ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openSnapshot">
        <div class="border-t border-white/5">
          <MVirtualList
            :items="snapshotVersions"
            :item-key="(v) => v.id"
            :estimate-size="49"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
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
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>

    <!-- Old Group -->
    <MCard v-if="oldVersions.length > 0" class="overflow-hidden">
      <button
        type="button"
        class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
        @click="openOld = !openOld"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-archive text-xl text-stone-400" />
          <span class="font-bold text-base"
            >{{ t('download.groups.old') }} ({{ oldVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
            openOld ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openOld">
        <div class="border-t border-white/5">
          <MVirtualList
            :items="oldVersions"
            :item-key="(v) => v.id"
            :estimate-size="49"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
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
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>

    <!-- April Fools Group -->
    <MCard v-if="aprilFoolsVersions.length > 0" class="overflow-hidden">
      <button
        type="button"
        class="w-full p-4 flex items-center justify-between text-left hover:bg-white/5 transition-colors cursor-pointer"
        @click="openAprilFools = !openAprilFools"
      >
        <div class="flex items-center gap-3">
          <span class="i-lucide-party-popper text-xl text-purple-400" />
          <span class="font-bold text-base"
            >{{ t('download.groups.aprilFools') }} ({{ aprilFoolsVersions.length }})</span
          >
        </div>
        <span
          :class="[
            'i-lucide-chevron-down text-lg text-fg-muted transition-transform duration-200',
            openAprilFools ? 'rotate-180' : '',
          ]"
        />
      </button>
      <MCollapse :open="openAprilFools">
        <div class="border-t border-white/5">
          <MVirtualList
            :items="aprilFoolsVersions"
            :item-key="(v) => v.id"
            :estimate-size="49"
            :overscan="10"
            max-height="65vh"
          >
            <template #item="{ item: v }">
              <div
                class="px-4 py-3 flex items-center justify-between hover:bg-white/5 transition-colors cursor-pointer group border-b border-white/5"
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
                  class="i-lucide-chevron-right text-fg-muted group-hover:text-primary transition-colors"
                />
              </div>
            </template>
          </MVirtualList>
        </div>
      </MCollapse>
    </MCard>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { CompProject } from '../../../ipc/client'
import {
  MCard,
  MBadge,
  MButton,
  MIconButton,
  MBrandIcon,
  compTypeMeta,
  type CompTypeKey,
  useToast,
} from '../../../ui'
import { useFormat } from '../../../composables/useFormat'

const props = defineProps<{
  project: CompProject
  isFavorite: boolean
  /** `settings.comp.showZhName`: render the Chinese mod-name database name as the title. */
  showZhName?: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle-favorite', project: CompProject): void
  (e: 'open-website', url?: string | null): void
  (e: 'add-to-preset'): void
}>()

const { t } = useI18n()
const toast = useToast()

/**
 * Presets hold community resources; a modpack is installed as a whole and a Modrinth world
 * does not exist, so neither can be added.
 */
const canAddToPreset = computed(
  () =>
    props.project.compType !== 'modpack' &&
    !(props.project.compType === 'world' && props.project.platform === 'modrinth'),
)

const displayTitle = computed(() =>
  props.showZhName && props.project.zhName ? props.project.zhName : props.project.title,
)

function openMcMod() {
  // A known wiki id is a direct hit; otherwise fall back to searching by the English title.
  const cleanTitle = props.project.title.split('|')[0]?.trim() || props.project.title
  const url = props.project.wikiId
    ? `https://www.mcmod.cn/class/${props.project.wikiId}.html`
    : `https://www.mcmod.cn/s?key=${encodeURIComponent(cleanTitle)}&site=all&filter=0`
  emit('open-website', url)
}

function copyName() {
  navigator.clipboard.writeText(props.project.title)
  toast.success(t('comp.project.copiedName'))
}

function copyLink() {
  const url = props.project.websiteUrl || `https://modrinth.com/mod/${props.project.id}`
  navigator.clipboard.writeText(url)
  toast.success(t('comp.project.copiedLink'))
}

// Region-aware formatting: culture comes from the language settings.
const { formatDate } = useFormat()
</script>

<template>
  <MCard class="p-6">
    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div class="flex items-start gap-4 min-w-0 flex-1">
        <MImage
          :src="project.iconUrl"
          :fallback-icon="compTypeMeta[project.compType as CompTypeKey]?.icon || 'i-lucide-box'"
          class="w-20 h-20 rounded-xl shrink-0"
        />

        <div class="min-w-0 flex-1">
          <div class="flex items-center gap-2.5 flex-wrap">
            <h2 class="text-2xl font-bold text-fg select-text leading-tight">
              {{ displayTitle }}
            </h2>
            <MBadge variant="neutral" size="sm" class="shrink-0">
              <MBrandIcon
                v-if="project.platform === 'modrinth' || project.platform === 'curseforge'"
                :name="project.platform"
                class="w-3.5 h-3.5 mr-1"
              />
              {{ project.platform }}
            </MBadge>
            <MBadge variant="neutral" size="sm" class="shrink-0">{{ project.compType }}</MBadge>
          </div>
          <div
            v-if="showZhName && project.zhName && project.zhName !== project.title"
            class="text-sm text-fg-muted mt-0.5 select-text"
            data-testid="comp-project-english-title"
          >
            {{ project.title }}
          </div>

          <div class="text-sm text-fg-muted mt-1 select-text">
            {{ t('comp.project.author', { author: project.author }) }}
          </div>

          <div class="flex items-center space-x-4 mt-2 text-xs text-fg-muted select-text">
            <span>{{
              t('comp.project.downloads', {
                count: project.downloads.toString(),
              })
            }}</span>
            <!-- Same rule as the card: a platform without follows must not show "0". -->
            <span v-if="project.follows > 0">{{
              t('comp.project.follows', { count: project.follows.toString() })
            }}</span>
            <span>{{
              t('comp.project.updated', {
                date: formatDate(project.updatedAt),
              })
            }}</span>
          </div>
        </div>
      </div>
      <!-- Right / Side Actions -->
      <div class="flex flex-wrap items-center gap-2 shrink-0 self-start md:self-center">
        <MButton
          v-if="project.websiteUrl"
          variant="tonal"
          size="sm"
          @click="emit('open-website', project.websiteUrl)"
        >
          <MBrandIcon
            v-if="project.platform === 'modrinth' || project.platform === 'curseforge'"
            :name="project.platform"
            class="w-3.5 h-3.5 mr-1.5"
          />
          <span v-else class="i-lucide-globe mr-1.5 w-3.5 h-3.5" />
          {{ t('comp.project.projectHome') }}
        </MButton>

        <MButton v-if="canAddToPreset" variant="tonal" size="sm" @click="emit('add-to-preset')">
          <span class="i-lucide-package-plus mr-1.5 w-3.5 h-3.5" />
          {{ t('comp.project.addToPreset') }}
        </MButton>

        <MButton variant="tonal" size="sm" @click="openMcMod">
          <span class="i-lucide-globe mr-1.5 w-3.5 h-3.5 text-primary" />
          {{ t('comp.project.mcmod') }}
        </MButton>

        <MButton
          :variant="isFavorite ? 'primary' : 'tonal'"
          size="sm"
          @click="emit('toggle-favorite', project)"
        >
          <span
            :class="isFavorite ? 'i-lucide-heart fill-current' : 'i-lucide-heart'"
            class="mr-1.5 w-3.5 h-3.5 text-rose-500"
          />
          {{ isFavorite ? t('comp.project.favorited') : t('comp.project.favorite') }}
        </MButton>

        <MIconButton
          icon="i-lucide-copy"
          variant="tonal"
          size="sm"
          :tooltip="t('comp.project.copyName')"
          @click="copyName"
        />

        <MIconButton
          icon="i-lucide-link"
          variant="tonal"
          size="sm"
          :tooltip="t('comp.project.copyLink')"
          @click="copyLink"
        />
      </div>
    </div>
  </MCard>
</template>

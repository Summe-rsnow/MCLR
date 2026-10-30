<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { CompProject } from '../../../ipc/client'
import {
  MCard,
  MBadge,
  MButton,
  MBrandIcon,
  MImage,
  compTypeMeta,
  type CompTypeKey,
  useToast,
} from '../../../ui'
import { useFormat } from '../../../composables/useFormat'

const props = defineProps<{
  project: CompProject
  isFavorite: boolean
}>()

const emit = defineEmits<{
  (e: 'toggle-favorite', project: CompProject): void
  (e: 'open-website', url?: string | null): void
}>()

const { t } = useI18n()
const toast = useToast()

function openMcMod() {
  const cleanTitle = props.project.title.split('|')[0]?.trim() || props.project.title
  const url = `https://www.mcmod.cn/s?key=${encodeURIComponent(cleanTitle)}&site=all&filter=0`
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
      <div class="flex items-start space-x-4">
        <MImage
          :src="project.iconUrl"
          :fallback-icon="compTypeMeta[project.compType as CompTypeKey]?.icon || 'i-lucide-box'"
          class="w-20 h-20 rounded-xl bg-sunken shrink-0"
        />

        <div>
          <div class="flex items-center space-x-3">
            <h2 class="text-2xl font-bold text-fg select-text">
              {{ project.title }}
            </h2>
            <MBadge variant="neutral">
              <MBrandIcon
                v-if="project.platform === 'modrinth' || project.platform === 'curseforge'"
                :name="project.platform"
                class="w-3.5 h-3.5 mr-1"
              />
              {{ project.platform }}
            </MBadge>
            <MBadge variant="neutral">{{ project.compType }}</MBadge>
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
            <span>{{ t('comp.project.follows', { count: project.follows.toString() }) }}</span>
            <span>{{
              t('comp.project.updated', {
                date: formatDate(project.updatedAt),
              })
            }}</span>
          </div>
        </div>
      </div>
      <!-- Right / Side Actions -->
      <div class="flex flex-wrap items-center gap-2 self-start md:self-auto">
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

        <MButton variant="tonal" size="sm" @click="openMcMod">
          <span class="i-lucide-globe mr-1.5 w-3.5 h-3.5 text-primary" />
          {{ t('comp.project.mcmod') }}
        </MButton>

        <MButton variant="tonal" size="sm" @click="copyName">
          <span class="i-lucide-copy mr-1.5 w-3.5 h-3.5" />
          {{ t('comp.project.copyName') }}
        </MButton>

        <MButton variant="tonal" size="sm" @click="copyLink">
          <span class="i-lucide-link mr-1.5 w-3.5 h-3.5" />
          {{ t('comp.project.copyLink') }}
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
      </div>
    </div>
  </MCard>
</template>

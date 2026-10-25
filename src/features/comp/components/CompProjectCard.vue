<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { CompProject, CompPlatform } from '../../../ipc/client'
import { MCard, MBadge, MIconButton, MBrandIcon, compTypeMeta, type CompTypeKey } from '../../../ui'

defineProps<{
  project: CompProject
  isFavorite: boolean
  activeTab: string
}>()

const emit = defineEmits<{
  (e: 'click', platform: CompPlatform, id: string): void
  (e: 'toggle-favorite', project: CompProject): void
}>()

const { t } = useI18n()

function formatNumber(num: number): string {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1) + 'M'
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'K'
  }
  return num.toString()
}
</script>

<template>
  <MCard
    hoverable
    class="cursor-pointer p-4 flex flex-col justify-between"
    @click="emit('click', project.platform, project.id)"
  >
    <div>
      <div class="flex items-start space-x-3">
        <img
          v-if="project.iconUrl"
          :src="project.iconUrl"
          class="w-14 h-14 rounded-xl object-cover bg-sunken shrink-0"
          alt=""
          loading="lazy"
          decoding="async"
        />
        <div
          v-else
          class="w-14 h-14 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
        >
          <span
            :class="compTypeMeta[activeTab as CompTypeKey]?.icon || 'i-lucide-box'"
            class="w-7 h-7 inline-block"
          />
        </div>

        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between">
            <h3 class="font-bold text-base text-fg truncate">{{ project.title }}</h3>
            <MIconButton
              :icon="
                isFavorite
                  ? 'i-lucide-heart fill-current text-rose-500'
                  : 'i-lucide-heart text-fg-muted hover:text-rose-500'
              "
              size="sm"
              :class="
                isFavorite
                  ? 'i-lucide-heart fill-current text-rose-500'
                  : 'i-lucide-heart text-fg-muted hover:text-rose-500'
              "
              @click.stop="emit('toggle-favorite', project)"
            />
          </div>
          <div class="text-xs text-fg-muted mt-0.5 truncate flex items-center gap-1.5">
            <MBrandIcon
              v-if="project.platform === 'modrinth' || project.platform === 'curseforge'"
              :name="project.platform"
              class="w-3 h-3 text-fg-muted"
            />
            <span>{{ t('comp.project.author', { author: project.author }) }}</span>
          </div>
        </div>
      </div>

      <div class="mt-3 line-clamp-2 h-8 leading-4 flex items-start">
        <span
          v-if="project.categories?.[0]"
          class="text-xs font-medium text-primary mr-2 shrink-0"
          >{{ project.categories[0] }}</span
        >
        <span class="text-xs text-fg-muted line-clamp-2">{{ project.summary }}</span>
      </div>
    </div>

    <div class="mt-4 pt-3 border-t border-line flex items-center justify-between gap-2 text-xs">
      <div class="flex items-center gap-3 shrink-0">
        <span class="text-xs text-fg-muted font-mono flex items-center gap-1">
          <i class="i-lucide-download w-3.5 h-3.5" />{{ formatNumber(project.downloads) }}
        </span>
        <span class="text-xs text-fg-muted flex items-center gap-1">
          <i class="i-lucide-clock w-3.5 h-3.5" />{{ project.updatedAt }}
        </span>
      </div>
      <div class="flex items-center gap-2 overflow-hidden justify-end">
        <span
          v-if="project.loaders?.length"
          class="text-xs text-fg-subtle truncate max-w-[120px]"
          >{{ project.loaders.join(', ') }}</span
        >
        <MBadge variant="neutral" size="sm">{{ project.platform }}</MBadge>
      </div>
    </div>
  </MCard>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { InstanceInfo } from '../../../ipc/client'
import { MCard, MLoaderIcon, MBadge } from '../../../ui'

defineProps<{
  instance: InstanceInfo
}>()

const emit = defineEmits<{
  (e: 'click', inst: InstanceInfo): void
  (e: 'contextmenu', ev: MouseEvent, inst: InstanceInfo): void
  (e: 'toggleStar', inst: InstanceInfo): void
  (e: 'quickAction', inst: InstanceInfo, tab: string): void
}>()

const { t } = useI18n()

function getLoaderBadgeVariant(kind?: string): 'primary' | 'warning' | 'neutral' {
  switch (kind) {
    case 'fabric':
      return 'primary'
    case 'forge':
    case 'neoforge':
      return 'warning'
    case 'cleanroom':
    case 'quilt':
    default:
      return 'neutral'
  }
}
</script>

<template>
  <MCard
    hoverable
    padding="sm"
    class="relative flex flex-col justify-between h-28 min-h-[112px] cursor-pointer group"
    @click="emit('click', instance)"
    @contextmenu="emit('contextmenu', $event, instance)"
  >
    <!-- Top Row Info -->
    <div class="flex items-start justify-between gap-2.5">
      <div class="flex items-center gap-3 min-w-0">
        <MLoaderIcon
          :kind="instance.kind || (instance.loaders?.[0]?.toLowerCase() ?? 'vanilla')"
          :icon="instance.config?.icon"
          :size="40"
          class="shrink-0"
        />
        <div class="min-w-0">
          <div
            class="font-bold text-sm text-fg truncate group-hover:text-primary transition-colors"
          >
            {{ instance.name }}
          </div>
          <div class="text-xs text-fg-muted font-mono truncate">
            {{ instance.mcVersion }}
          </div>
        </div>
      </div>

      <!-- Star / More Options -->
      <div class="flex items-center gap-1">
        <button
          type="button"
          class="p-1 transition-colors rounded-lg hover:bg-sunken cursor-pointer"
          :class="
            instance.config?.starred || instance.config?.pinned
              ? 'text-amber-400'
              : 'text-fg-muted/40 hover:text-fg-muted'
          "
          :title="
            instance.config?.starred || instance.config?.pinned
              ? t('instances.actions.unstar')
              : t('instances.actions.star')
          "
          @click.stop="emit('toggleStar', instance)"
        >
          <span
            class="w-4 h-4"
            :class="
              instance.config?.starred || instance.config?.pinned
                ? 'i-lucide-star fill-current'
                : 'i-lucide-star'
            "
          />
        </button>

        <button
          type="button"
          class="p-1 text-fg-muted hover:text-fg transition-colors rounded-lg hover:bg-sunken cursor-pointer"
          :title="t('instances.card.more')"
          @click.stop="emit('contextmenu', $event, instance)"
        >
          <span class="w-4 h-4 i-lucide-more-vertical" />
        </button>
      </div>
    </div>

    <!-- Badges & Quick Actions Row -->
    <div class="flex items-center justify-between gap-2 mt-auto pt-1.5">
      <div class="flex flex-wrap items-center gap-1.5 min-w-0">
        <MBadge
          v-if="instance.loaders && instance.loaders.length > 0"
          v-for="loader in instance.loaders"
          :key="loader"
          :variant="getLoaderBadgeVariant(instance.kind)"
          size="sm"
        >
          {{ loader }}
        </MBadge>
        <MBadge
          v-else-if="instance.kind && instance.kind !== 'vanilla'"
          variant="neutral"
          size="sm"
        >
          {{ instance.kind }}
        </MBadge>
        <MBadge v-if="instance.state !== 'ok'" variant="danger" size="sm">
          {{ t(`instances.card.state.${instance.state}`) }}
        </MBadge>
        <MBadge v-if="instance.config?.isolation === 'on'" variant="info" size="sm">
          {{ t('instances.actions.isolationOn') }}
        </MBadge>
      </div>

      <!-- Quick Entry Buttons -->
      <div v-if="instance.modable" class="flex items-center gap-1 shrink-0">
        <button
          type="button"
          class="p-1 rounded text-fg-muted hover:text-primary hover:bg-primary/10 transition-colors cursor-pointer"
          :title="t('instances.actions.modsQuick')"
          @click.stop="emit('quickAction', instance, 'mods')"
        >
          <span class="w-3.5 h-3.5 i-lucide-puzzle" />
        </button>
        <button
          type="button"
          class="p-1 rounded text-fg-muted hover:text-primary hover:bg-primary/10 transition-colors cursor-pointer"
          :title="t('instances.actions.resourcepacksQuick')"
          @click.stop="emit('quickAction', instance, 'resourcepacks')"
        >
          <span class="w-3.5 h-3.5 i-lucide-palette" />
        </button>
        <button
          type="button"
          class="p-1 rounded text-fg-muted hover:text-primary hover:bg-primary/10 transition-colors cursor-pointer"
          :title="t('instances.actions.screenshotsQuick')"
          @click.stop="emit('quickAction', instance, 'screenshots')"
        >
          <span class="w-3.5 h-3.5 i-lucide-image" />
        </button>
      </div>
    </div>
  </MCard>
</template>

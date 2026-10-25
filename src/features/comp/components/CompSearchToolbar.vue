<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { CompPlatform, CompSort } from '../../../ipc/client'
import { MCard, MSelect, MIconButton, MInput } from '../../../ui'

export interface SelectOption {
  label: string
  value: string
}

defineProps<{
  platform: CompPlatform | 'all'
  category: string | null
  sort: CompSort
  targetInstanceId: string
  mcVersion: string | null
  loader: string | null
  loading: boolean
  platformOptions: SelectOption[]
  categoryOptions: SelectOption[]
  sortOptions: SelectOption[]
  targetInstanceOptions: SelectOption[]
  mcVersionOptions: SelectOption[]
  loaderOptions: SelectOption[]
  searchPlaceholder: string
  compType?: string
}>()

const query = defineModel<string>('query', { default: '' })

const emit = defineEmits<{
  (e: 'update:platform', val: CompPlatform | 'all'): void
  (e: 'update:category', val: string): void
  (e: 'update:sort', val: CompSort): void
  (e: 'update:target-instance', val: string): void
  (e: 'update:mc-version', val: string): void
  (e: 'update:loader', val: string): void
  (e: 'refresh'): void
  (e: 'search'): void
}>()

const { t } = useI18n()
</script>

<template>
  <MCard class="p-2.5 flex flex-col gap-2 shadow-sm">
    <!-- Row 1: Filters -->
    <div class="flex flex-wrap items-center gap-2">
      <!-- Platform -->
      <div class="w-32 shrink-0">
        <MSelect
          :model-value="platform"
          :options="platformOptions"
          size="sm"
          @update:model-value="(val) => emit('update:platform', val as CompPlatform | 'all')"
        />
      </div>

      <!-- Category -->
      <div class="w-36 shrink-0">
        <MSelect
          :model-value="category || ''"
          :options="categoryOptions"
          size="sm"
          @update:model-value="(val) => emit('update:category', String(val))"
        />
      </div>

      <!-- Sort -->
      <div class="w-36 shrink-0">
        <MSelect
          :model-value="sort"
          :options="sortOptions"
          size="sm"
          @update:model-value="(val) => emit('update:sort', val as CompSort)"
        />
      </div>

      <!-- Target Instance / Version (Hidden for Modpack since modpacks create new instances) -->
      <div v-if="compType !== 'modpack'" class="flex items-center gap-1.5 shrink-0">
        <MSelect
          :model-value="targetInstanceId"
          :options="targetInstanceOptions"
          size="sm"
          class="w-48"
          @update:model-value="(val) => emit('update:target-instance', String(val))"
        />
        <MSelect
          :model-value="mcVersion || ''"
          :options="mcVersionOptions"
          size="sm"
          class="w-36"
          @update:model-value="(val) => emit('update:mc-version', String(val))"
        />
      </div>

      <!-- Loader -->
      <div class="w-36 shrink-0">
        <MSelect
          :model-value="loader || ''"
          :options="loaderOptions"
          size="sm"
          @update:model-value="(val) => emit('update:loader', String(val))"
        />
      </div>

      <!-- Refresh button -->
      <MIconButton
        icon="i-lucide-rotate-cw"
        size="sm"
        variant="tonal"
        :loading="loading"
        :title="t('common.refresh')"
        @click="emit('refresh')"
      />
    </div>

    <!-- Row 2: Search Bar -->
    <div class="w-full">
      <MInput
        v-model="query"
        :placeholder="searchPlaceholder"
        :icon="loading ? 'i-lucide-loader-2 animate-spin' : 'i-lucide-search'"
        clearable
        @keydown.enter="emit('search')"
      />
    </div>
  </MCard>
</template>

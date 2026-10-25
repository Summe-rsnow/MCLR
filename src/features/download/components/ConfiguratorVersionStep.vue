<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { MBadge, MButton } from '../../../ui'
import { dropOf } from '../compat'

defineProps<{
  mcVersion: string
  mode: 'create' | 'modify'
}>()

const emit = defineEmits<{
  (e: 'back'): void
}>()

const { t } = useI18n()
</script>

<template>
  <div class="flex items-center justify-between p-4 rounded-2xl bg-white/5 border border-white/10">
    <div class="flex items-center gap-3">
      <span class="i-lucide-box text-2xl text-primary" />
      <div>
        <div class="text-xs text-fg-muted">
          {{ t('download.configurator.targetVersion') }}
        </div>
        <div class="text-lg font-bold flex items-center gap-2">
          {{ mcVersion }}
          <MBadge variant="primary" size="sm"> Drop {{ dropOf(mcVersion) }} </MBadge>
        </div>
      </div>
    </div>
    <MButton v-if="mode === 'create'" variant="text" size="sm" @click="emit('back')">
      <span class="i-lucide-refresh-cw mr-1.5" />
      {{ t('download.configurator.changeVersion') }}
    </MButton>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { MInput, MBadge, MButton } from '../../../ui'

defineProps<{
  mode: 'create' | 'modify'
  instanceName: string
  isNameManuallyEdited: boolean
  nameValidationError: string | null
  isQuiltInstance: boolean
  initialSummary: string
  currentSelectionSummary: string
  submitting: boolean
  isModified: boolean
  loadingPrimaryVersions: boolean
}>()

const emit = defineEmits<{
  (e: 'update:instance-name', val: string): void
  (e: 'input-name', val: string): void
  (e: 'reset-auto-name'): void
  (e: 'install'): void
}>()

const { t } = useI18n()
</script>

<template>
  <div class="flex flex-col gap-6">
    <!-- Section 5: Instance Name Input (Create mode only) -->
    <div v-if="mode === 'create'" class="flex flex-col gap-2">
      <div class="flex items-center justify-between text-sm">
        <label class="font-medium text-fg-muted">
          {{ t('download.configurator.instanceName') }}
        </label>
        <div class="flex items-center gap-2">
          <MBadge v-if="!isNameManuallyEdited" size="sm" variant="neutral">
            {{ t('download.configurator.autoNamed') }}
          </MBadge>
          <MButton v-else variant="text" size="xs" @click="emit('reset-auto-name')">
            {{ t('download.configurator.autoNamed') }}
          </MButton>
        </div>
      </div>
      <MInput
        :model-value="instanceName"
        :placeholder="t('download.configurator.namePlaceholder')"
        @update:model-value="(val) => emit('input-name', val)"
      />
      <div v-if="nameValidationError" class="text-xs text-red-400">
        {{ t(nameValidationError) }}
      </div>
    </div>

    <!-- Section 6: Diff in Modify mode -->
    <div
      v-if="mode === 'modify' && !isQuiltInstance"
      class="p-4 rounded-xl border border-white/10 bg-white/5 flex flex-col gap-1.5"
    >
      <div class="text-xs text-fg-muted">
        {{ t('download.modify.diffLabel') }}
      </div>
      <div class="text-sm font-medium flex items-center gap-2">
        <span>{{ initialSummary || 'Vanilla' }}</span>
        <span class="i-lucide-arrow-right text-xs text-fg-muted" />
        <span class="text-primary">{{ currentSelectionSummary }}</span>
      </div>
    </div>

    <!-- Sticky Bottom Action Bar -->
    <div
      class="sticky bottom-0 -mx-4 -mb-4 p-4 bg-raised/80 backdrop-blur-md border-t border-white/10 flex items-center justify-between rounded-b-2xl z-20 m-slide-up"
    >
      <div class="flex flex-col">
        <div class="text-xs text-fg-muted">{{ t('download.configurator.summaryLabel') }}</div>
        <div class="text-sm font-semibold text-primary truncate max-w-xs sm:max-w-md">
          {{ currentSelectionSummary }}
        </div>
      </div>

      <MButton
        variant="primary"
        size="md"
        :loading="submitting"
        :disabled="
          (mode === 'create' && Boolean(nameValidationError)) ||
          (mode === 'modify' && isQuiltInstance) ||
          loadingPrimaryVersions
        "
        @click="emit('install')"
      >
        <span :class="[mode === 'modify' ? 'i-lucide-wrench' : 'i-lucide-download', 'mr-2']" />
        {{
          mode === 'modify'
            ? isModified
              ? t('download.configurator.modifyAction')
              : t('download.configurator.resetAction')
            : t('download.configurator.installAction')
        }}
      </MButton>
    </div>
  </div>
</template>

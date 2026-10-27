<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { MInput, MBadge, MButton, MHelpTooltip } from '../../../ui'
import { useErrorText } from '../../../composables/useErrorText'

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
// Validation codes are error codes (`instance.name.duplicate`), which live under
// the `error.` namespace; formatting them directly printed the raw key.
const { formatError } = useErrorText()
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
      <div v-if="nameValidationError" class="text-xs text-red-400" data-testid="name-error">
        {{ formatError({ code: nameValidationError }) }}
      </div>
    </div>

    <!-- Sticky Bottom Action Bar -->
    <div
      class="p-4 bg-raised border border-line flex items-center justify-between rounded-xl shadow-sm mt-3"
    >
      <div class="flex flex-col min-w-0">
        <div class="text-xs text-fg-muted flex items-center gap-1">
          <span>{{ t('download.configurator.summaryLabel') }}</span>
          <!-- Reuses the shared hint component; only the icon (and tooltip target)
               differs, so the summary text itself stays plain. -->
          <MHelpTooltip
            v-if="mode === 'modify' && isModified && !isQuiltInstance"
            :text="`${initialSummary || 'Vanilla'} → ${currentSelectionSummary}`"
            icon="i-lucide-circle-alert"
            size="xs"
            data-testid="config-diff-hint"
          />
        </div>
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

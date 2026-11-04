<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MButton from './MButton.vue'
import MOverlay from './MOverlay.vue'

const { t } = useI18n()

const props = withDefaults(
  defineProps<{
    open: boolean
    title?: string
    confirmText?: string
    cancelText?: string
    hideFooter?: boolean
    confirmVariant?: 'primary' | 'danger' | 'tonal'
    confirmLoading?: boolean
    confirmDisabled?: boolean
    maxWidth?: 'sm' | 'md' | 'lg' | 'xl' | '2xl'
  }>(),
  {
    title: undefined,
    confirmText: undefined,
    cancelText: undefined,
    hideFooter: false,
    confirmVariant: 'primary',
    confirmLoading: false,
    confirmDisabled: false,
    maxWidth: 'md',
  },
)

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'confirm'): void
  (e: 'cancel'): void
}>()

const maxWidthClass = computed(() => {
  switch (props.maxWidth) {
    case 'sm':
      return 'max-w-sm'
    case 'lg':
      return 'max-w-lg'
    case 'xl':
      return 'max-w-xl'
    case '2xl':
      return 'max-w-2xl'
    default:
      return 'max-w-md'
  }
})

function onCancel() {
  emit('update:open', false)
  emit('cancel')
}

function onConfirm() {
  emit('confirm')
}
</script>

<template>
  <MOverlay :open="open" transition="m-modal" @close="onCancel">
    <div
      class="m-overlay m-refract w-full p-6 rounded-2xl flex flex-col gap-4 text-fg shadow-2xl"
      :class="maxWidthClass"
      role="dialog"
      aria-modal="true"
    >
      <div v-if="title || $slots.header" class="flex items-center justify-between gap-3">
        <slot name="header">
          <h3 class="text-lg font-bold text-fg">{{ title }}</h3>
        </slot>
        <button
          :aria-label="t('common.close')"
          type="button"
          class="text-fg-muted hover:text-fg hover:bg-sunken cursor-pointer p-1.5 rounded-lg transition-colors"
          @click="onCancel"
        >
          <span class="i-lucide-x w-4 h-4 block" />
        </button>
      </div>

      <div class="text-sm text-fg-muted select-text">
        <slot />
      </div>

      <div v-if="!hideFooter" class="flex items-center justify-end gap-2 pt-2">
        <slot name="footer">
          <MButton variant="text" @click="onCancel">
            {{ cancelText || t('common.cancel') }}
          </MButton>
          <MButton
            :variant="confirmVariant"
            :loading="confirmLoading"
            :disabled="confirmDisabled"
            @click="onConfirm"
          >
            {{ confirmText || t('common.confirm') }}
          </MButton>
        </slot>
      </div>
    </div>
  </MOverlay>
</template>

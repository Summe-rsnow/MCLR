<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import MButton from './MButton.vue'
import MOverlay from './MOverlay.vue'

let t = (k: string) => k
try {
  const i18n = useI18n()
  t = i18n.t
} catch {}

const props = withDefaults(
  defineProps<{
    open: boolean
    title?: string
    subtitle?: string
    placement?: 'right' | 'left' | 'bottom'
    size?: 'sm' | 'md' | 'lg' | 'xl'
    hideFooter?: boolean
    confirmText?: string
    cancelText?: string
    confirmVariant?: 'primary' | 'danger' | 'tonal'
    confirmLoading?: boolean
    confirmDisabled?: boolean
  }>(),
  {
    title: undefined,
    subtitle: undefined,
    placement: 'right',
    size: 'md',
    hideFooter: true,
    confirmText: undefined,
    cancelText: undefined,
    confirmVariant: 'primary',
    confirmLoading: false,
    confirmDisabled: false,
  },
)

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'close'): void
  (e: 'confirm'): void
}>()

function onClose() {
  emit('update:open', false)
  emit('close')
}

function onConfirm() {
  emit('confirm')
}

const overlayTransition = computed(() => {
  switch (props.placement) {
    case 'left':
      return 'm-drawer-left'
    case 'bottom':
      return 'm-drawer-bottom'
    case 'right':
    default:
      return 'm-drawer-right'
  }
})

const sizeClass = computed(() => {
  if (props.placement === 'bottom') {
    switch (props.size) {
      case 'sm':
        return 'w-full max-h-[360px]'
      case 'lg':
        return 'w-full max-h-[560px]'
      case 'xl':
        return 'w-full max-h-[680px]'
      case 'md':
      default:
        return 'w-full max-h-[448px]'
    }
  }

  switch (props.size) {
    case 'sm':
      return 'w-[360px] max-w-full h-full'
    case 'lg':
      return 'w-[560px] max-w-full h-full'
    case 'xl':
      return 'w-[680px] max-w-full h-full'
    case 'md':
    default:
      return 'w-[448px] max-w-full h-full'
  }
})

const placementClass = computed(() => {
  switch (props.placement) {
    case 'left':
      return 'border-r border-line'
    case 'bottom':
      return 'border-t border-line rounded-t-2xl'
    case 'right':
    default:
      return 'border-l border-line'
  }
})
</script>

<template>
  <MOverlay :open="open" :align="placement" :transition="overlayTransition" @close="onClose">
    <div
      class="m-overlay m-refract flex flex-col text-fg shadow-2xl relative"
      :class="[sizeClass, placementClass]"
      role="dialog"
      aria-modal="true"
      data-drawer-panel="true"
    >
      <!-- Header -->
      <div
        v-if="title || subtitle || $slots.header"
        class="p-5 border-b border-line flex items-start justify-between gap-3 shrink-0"
      >
        <slot name="header">
          <div class="space-y-0.5 min-w-0">
            <h3 class="text-base font-bold text-fg truncate">{{ title }}</h3>
            <p v-if="subtitle" class="text-xs text-fg-muted truncate">{{ subtitle }}</p>
          </div>
        </slot>
        <button
          type="button"
          class="text-fg-muted hover:text-fg hover:bg-sunken cursor-pointer p-1.5 rounded-lg transition-colors shrink-0"
          @click="onClose"
        >
          <span class="i-lucide-x w-4 h-4 block" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-5 flex-1 min-h-0 overflow-y-auto select-text">
        <slot />
      </div>

      <!-- Footer -->
      <div
        v-if="!hideFooter"
        class="p-4 border-t border-line flex items-center justify-end gap-2 shrink-0"
      >
        <slot name="footer">
          <MButton variant="text" @click="onClose">
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

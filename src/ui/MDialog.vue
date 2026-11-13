<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Dialog } from '@miuui/vue'
import MButton from './MButton.vue'

let t = (key: string) => key
try {
  const i18n = useI18n()
  t = i18n.t
} catch {
  // Mounted without vue-i18n (unit tests)
}

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

const dialogWidth = computed(() => {
  switch (props.maxWidth) {
    case 'sm':
      return '384px'
    case 'lg':
      return '512px'
    case 'xl':
      return '576px'
    case '2xl':
      return '672px'
    case 'md':
    default:
      return '448px'
  }
})

function onUpdateOpen(next: boolean) {
  emit('update:open', next)
  if (!next) emit('cancel')
}

function onCancel() {
  emit('update:open', false)
  emit('cancel')
}

function onConfirm() {
  emit('confirm')
}
</script>

<template>
  <Dialog
    :open="open"
    :title="title"
    :width="dialogWidth"
    :hide-header="!title && !$slots.header"
    :aria-label="title || 'Dialog'"
    :class-names="{
      content: 'm-overlay m-refract',
      body: 'select-text text-sm text-fg-muted',
    }"
    @update:open="onUpdateOpen"
  >
    <template v-if="$slots.header" #title>
      <slot name="header" />
    </template>
    <slot />
    <template v-if="!hideFooter" #footer>
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
    </template>
  </Dialog>
</template>

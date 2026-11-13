<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { Drawer } from '@miuui/vue'
import type { DrawerSide } from '@miuui/vue'
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
    subtitle?: string
    placement?: 'right' | 'left' | 'bottom'
    size?: 'sm' | 'md' | 'lg' | 'xl'
    hideFooter?: boolean
    confirmText?: string
    cancelText?: string
    confirmLoading?: boolean
    confirmDisabled?: boolean
    confirmVariant?: 'primary' | 'danger' | 'tonal'
  }>(),
  {
    title: undefined,
    subtitle: undefined,
    placement: 'right',
    size: 'md',
    hideFooter: true,
    confirmText: undefined,
    cancelText: undefined,
    confirmLoading: false,
    confirmDisabled: false,
    confirmVariant: 'primary',
  },
)

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'confirm'): void
  (e: 'close'): void
}>()

const side = computed<DrawerSide>(() => {
  if (props.placement === 'left') return 'start'
  if (props.placement === 'bottom') return 'bottom'
  return 'end'
})

const width = computed(() => {
  if (props.placement === 'bottom') return undefined
  switch (props.size) {
    case 'sm':
      return '360px'
    case 'lg':
      return '560px'
    case 'xl':
      return '680px'
    case 'md':
    default:
      return '448px'
  }
})

function onUpdateOpen(next: boolean) {
  emit('update:open', next)
  if (!next) emit('close')
}

function onClose() {
  emit('update:open', false)
  emit('close')
}
</script>

<template>
  <Drawer
    :open="open"
    :side="side"
    :width="width"
    :title="title"
    :class-names="{
      content: 'm-drawer',
      body: 'select-text',
    }"
    @update:open="onUpdateOpen"
  >
    <template v-if="title || subtitle || $slots.header" #header>
      <slot name="header">
        <div class="space-y-0.5 min-w-0">
          <h3 v-if="title" class="text-base font-bold text-fg truncate">{{ title }}</h3>
          <p v-if="subtitle" class="text-xs text-fg-muted truncate">{{ subtitle }}</p>
        </div>
      </slot>
    </template>
    <slot />
    <template v-if="!hideFooter" #footer>
      <slot name="footer">
        <MButton variant="text" @click="onClose">
          {{ cancelText || t('common.cancel') }}
        </MButton>
        <MButton
          :variant="confirmVariant"
          :loading="confirmLoading"
          :disabled="confirmDisabled"
          @click="emit('confirm')"
        >
          {{ confirmText || t('common.confirm') }}
        </MButton>
      </slot>
    </template>
  </Drawer>
</template>

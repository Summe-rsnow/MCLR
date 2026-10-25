<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import MButton from './MButton.vue'

const { t } = useI18n()

withDefaults(
  defineProps<{
    open: boolean
    title?: string
    confirmText?: string
    cancelText?: string
    hideFooter?: boolean
  }>(),
  {
    title: undefined,
    confirmText: undefined,
    cancelText: undefined,
    hideFooter: false,
  },
)

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'confirm'): void
  (e: 'cancel'): void
}>()

function onCancel() {
  emit('update:open', false)
  emit('cancel')
}

function onConfirm() {
  emit('confirm')
}
</script>

<template>
  <Teleport to="body">
    <Transition name="m-modal">
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/45 backdrop-blur-xs"
      @click.self="onCancel"
    >
      <div
        class="m-overlay m-refract w-full max-w-md p-6 rounded-2xl flex flex-col gap-4 text-fg"
        role="dialog"
      >
        <div v-if="title || $slots.header" class="flex items-center justify-between gap-3">
          <slot name="header">
            <h3 class="text-lg font-bold text-fg">{{ title }}</h3>
          </slot>
          <button
            type="button"
            class="text-fg-muted hover:text-fg hover:bg-sunken cursor-pointer p-1.5 rounded-lg transition-colors"
            @click="onCancel"
          >
            <span class="i-lucide-x w-4 h-4 block" />
          </button>
        </div>

        <div class="text-sm text-fg-muted">
          <slot />
        </div>

        <div v-if="!hideFooter" class="flex items-center justify-end gap-2 pt-2">
          <slot name="footer">
            <MButton variant="text" @click="onCancel">
              {{ cancelText || t('common.cancel') }}
            </MButton>
            <MButton variant="primary" @click="onConfirm">
              {{ confirmText || t('common.confirm') }}
            </MButton>
          </slot>
        </div>
      </div>
    </div>
    </Transition>
  </Teleport>
</template>

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
    <div
      v-if="open"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/40 backdrop-blur-sm"
      @click.self="onCancel"
    >
      <div
        class="m-card w-full max-w-md p-6 rounded-2xl shadow-xl flex flex-col gap-4 bg-surface text-text"
        role="dialog"
      >
        <div v-if="title || $slots.header" class="flex items-center justify-between">
          <h3 class="text-lg font-semibold">{{ title }}</h3>
          <button
            type="button"
            class="text-textMuted hover:text-text cursor-pointer p-1 rounded-lg"
            @click="onCancel"
          >
            <span class="i-lucide-x w-4 h-4 block" />
          </button>
        </div>

        <div class="text-sm text-textMuted">
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
  </Teleport>
</template>

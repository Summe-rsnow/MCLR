<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { formatBytes } from '@/shared/format'
import { ipcClient, type TaskLeftovers } from '../../../ipc/client'
import { useDialog } from '../../../ui/useDialog'
import { useToast } from '../../../ui/useToast'
import { useErrorText } from '../../../composables/useErrorText'
import { MDialog, MButton, MCard } from '../../../ui'

const props = defineProps<{
  open: boolean
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
}>()

const { t } = useI18n()
const dialog = useDialog()
const toast = useToast()
const { formatError } = useErrorText()

const loading = ref(false)
const cleaning = ref(false)
const leftovers = ref<TaskLeftovers>({ bytes: 0, files: 0 })

async function loadInfo() {
  loading.value = true
  try {
    leftovers.value = await ipcClient.tasksLeftoversInfo()
  } catch (err) {
    console.error('Failed to get tasks leftovers info:', err)
  } finally {
    loading.value = false
  }
}

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) {
      loadInfo()
    }
  },
  { immediate: true },
)

async function onClean() {
  const confirmed = await dialog.confirmDanger({
    title: t('tasks.cleanup.confirmTitle'),
    content: t('tasks.cleanup.confirmContent', {
      count: leftovers.value.files,
      size: formatBytes(leftovers.value.bytes),
    }),
    confirmText: t('tasks.cleanup.cleanNow'),
    cancelText: t('common.cancel'),
  })
  if (!confirmed) return

  cleaning.value = true
  try {
    const report = await ipcClient.tasksCleanupLeftovers()
    toast.success(
      t('tasks.cleanup.cleaned') + ` (${report.files} files, ${formatBytes(report.bytes)})`,
    )
    emit('update:open', false)
  } catch (err) {
    toast.danger(formatError(err))
  } finally {
    cleaning.value = false
  }
}
</script>

<template>
  <MDialog
    :open="open"
    :title="t('tasks.cleanup.title')"
    max-width="sm"
    @update:open="(val) => emit('update:open', val)"
  >
    <div class="space-y-4 py-2">
      <p class="text-xs text-fg-muted leading-relaxed">
        {{ t('tasks.cleanup.desc') }}
      </p>

      <MCard padding="md" class="m-well space-y-2">
        <div class="flex items-center justify-between text-sm">
          <span class="text-fg-muted">{{
            t('tasks.cleanup.info', { count: leftovers.files, size: formatBytes(leftovers.bytes) })
          }}</span>
        </div>
      </MCard>

      <div class="flex justify-end gap-2 pt-2">
        <MButton variant="text" size="sm" @click="emit('update:open', false)">
          {{ t('common.cancel') }}
        </MButton>
        <MButton
          variant="danger"
          size="sm"
          icon="i-lucide-trash"
          :loading="cleaning || loading"
          :disabled="leftovers.files === 0"
          @click="onClean"
        >
          {{ t('tasks.cleanup.cleanNow') }}
        </MButton>
      </div>
    </div>
  </MDialog>
</template>

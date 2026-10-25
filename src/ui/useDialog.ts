import { ref } from 'vue'

export interface DialogOptions {
  title?: string
  content: string
  detail?: string
  severity?: 'info' | 'warning' | 'danger'
  confirmText?: string
  cancelText?: string
  confirmVariant?: 'primary' | 'danger' | 'tonal'
  requireCheckbox?: string
}

export interface ActiveDialogState {
  open: boolean
  title?: string
  content: string
  detail?: string
  severity?: 'info' | 'warning' | 'danger'
  confirmText?: string
  cancelText?: string
  confirmVariant?: 'primary' | 'danger' | 'tonal'
  requireCheckbox?: string
  resolve: (value: boolean) => void
}

const activeDialog = ref<ActiveDialogState | null>(null)

export function useDialog() {
  function confirm(options: DialogOptions): Promise<boolean> {
    const { promise, resolve } = Promise.withResolvers<boolean>()

    const severity = options.severity ?? 'warning'
    const confirmVariant = options.confirmVariant ?? (severity === 'danger' ? 'danger' : 'primary')

    activeDialog.value = {
      open: true,
      title: options.title,
      content: options.content,
      detail: options.detail,
      severity,
      confirmText: options.confirmText,
      cancelText: options.cancelText,
      confirmVariant,
      requireCheckbox: options.requireCheckbox,
      resolve,
    }
    return promise
  }

  function confirmDanger(
    options: Omit<DialogOptions, 'severity' | 'confirmVariant'>,
  ): Promise<boolean> {
    return confirm({
      ...options,
      severity: 'danger',
      confirmVariant: 'danger',
    })
  }

  function handleConfirm() {
    if (activeDialog.value) {
      activeDialog.value.resolve(true)
      activeDialog.value.open = false
      activeDialog.value = null
    }
  }

  function handleCancel() {
    if (activeDialog.value) {
      activeDialog.value.resolve(false)
      activeDialog.value.open = false
      activeDialog.value = null
    }
  }

  return {
    activeDialog,
    confirm,
    confirmDanger,
    handleConfirm,
    handleCancel,
  }
}

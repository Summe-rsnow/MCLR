import { ref } from 'vue'

export interface DialogOptions {
  title?: string
  content: string
  confirmText?: string
  cancelText?: string
}

const activeDialog = ref<{
  open: boolean
  title?: string
  content: string
  confirmText?: string
  cancelText?: string
  resolve: (value: boolean) => void
} | null>(null)

export function useDialog() {
  function confirm(options: DialogOptions): Promise<boolean> {
    let resolver: (value: boolean) => void = () => {}
    const promise = new Promise<boolean>((resolve) => {
      resolver = resolve
    })

    activeDialog.value = {
      open: true,
      title: options.title,
      content: options.content,
      confirmText: options.confirmText,
      cancelText: options.cancelText,
      resolve: resolver,
    }
    return promise
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
    handleConfirm,
    handleCancel,
  }
}

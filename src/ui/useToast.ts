import { computed } from 'vue'
import { useToast as useMiuToast, type ToastVariant } from '@miuui/vue'

export type ToastType = 'info' | 'success' | 'warning' | 'danger'

export interface ToastItem {
  id: number
  message: string
  type: ToastType
  duration?: number
}

function toMiuVariant(type: ToastType | undefined): ToastVariant {
  if (type === 'danger') return 'error'
  return type ?? 'info'
}

function fromMiuVariant(variant: ToastVariant): ToastType {
  if (variant === 'error') return 'danger'
  if (variant === 'default') return 'info'
  return variant
}

export function useToast() {
  const { toasts: miuToasts, toast: miuToast, dismiss } = useMiuToast()

  const toasts = computed<ToastItem[]>(() =>
    miuToasts.value.map((item) => ({
      id: item.id,
      message: item.title,
      type: fromMiuVariant(item.variant),
      duration: item.duration,
    })),
  )

  function show(toast: { message: string; type?: ToastType; duration?: number }) {
    return miuToast({
      title: toast.message,
      variant: toMiuVariant(toast.type),
      duration: toast.duration ?? 3000,
    })
  }

  function remove(id: number) {
    dismiss(id)
  }

  function info(message: string, duration?: number) {
    return show({ message, type: 'info', duration })
  }

  function success(message: string, duration?: number) {
    return show({ message, type: 'success', duration })
  }

  function warning(message: string, duration?: number) {
    return show({ message, type: 'warning', duration })
  }

  function danger(message: string, duration?: number) {
    return show({ message, type: 'danger', duration })
  }

  return {
    toasts,
    show,
    remove,
    info,
    success,
    warning,
    danger,
  }
}

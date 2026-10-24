import { ref } from 'vue'

export type ToastType = 'info' | 'success' | 'warning' | 'danger'

export interface ToastItem {
  id: number
  message: string
  type: ToastType
  duration?: number
}

const toasts = ref<ToastItem[]>([])
let nextId = 1

export function useToast() {
  function show(toast: { message: string; type?: ToastType; duration?: number }) {
    const id = nextId++
    const item: ToastItem = {
      id,
      message: toast.message,
      type: toast.type ?? 'info',
      duration: toast.duration ?? 3000,
    }
    toasts.value.push(item)

    const dur = item.duration ?? 3000
    if (dur > 0) {
      setTimeout(() => {
        remove(id)
      }, dur)
    }

    return id
  }

  function remove(id: number) {
    const index = toasts.value.findIndex((t) => t.id === id)
    if (index !== -1) {
      toasts.value.splice(index, 1)
    }
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

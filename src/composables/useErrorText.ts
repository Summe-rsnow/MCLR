import { useI18n } from 'vue-i18n'
import { AppErrorEx } from '../ipc/client'
import type { AppError } from '../ipc/bindings'

export function useErrorText() {
  const { t, te } = useI18n()

  function formatError(error: unknown): string {
    if (!error) {
      return t('error.unknown')
    }

    let code = 'unknown'
    let params: Record<string, string> = {}

    if (error instanceof AppErrorEx) {
      code = error.code
      params = { ...error.params }
      // `{detail}` in locale strings reads the server-side detail, when present.
      if (error.detail && !params.detail) params.detail = error.detail
    } else if (typeof error === 'object' && error !== null && 'code' in error) {
      const err = error as AppError
      code = err.code
      if (err.params) {
        for (const [k, v] of Object.entries(err.params)) {
          if (typeof v === 'string') params[k] = v
        }
      }
      if (err.detail && !params.detail) params.detail = err.detail
    } else if (error instanceof Error) {
      return error.message
    } else if (typeof error === 'string') {
      return error
    }

    const key = `error.${code}`
    if (te(key)) {
      return t(key, params)
    }

    return t('error.unknown')
  }

  return {
    formatError,
  }
}

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import MIconButton from './MIconButton.vue'

const props = withDefaults(
  defineProps<{
    page: number
    pageCount?: number
    hasNext?: boolean
    loading?: boolean
    teleport?: boolean
  }>(),
  {
    pageCount: undefined,
    hasNext: true,
    loading: false,
    teleport: true,
  },
)

const emit = defineEmits<{
  (e: 'update:page', page: number): void
}>()

const { t } = useI18n()
const dock = ref<HTMLElement | null>(null)

onMounted(() => {
  dock.value = document.getElementById('m-float-dock')
})

const items = computed<(number | '…')[]>(() => {
  if (props.pageCount === undefined) {
    return []
  }
  const total = props.pageCount
  const current = props.page
  if (total <= 7) {
    return Array.from({ length: total }, (_, i) => i + 1)
  }
  if (current <= 4) {
    return [1, 2, 3, 4, 5, '…', total]
  }
  if (current >= total - 3) {
    return [1, '…', total - 4, total - 3, total - 2, total - 1, total]
  }
  return [1, '…', current - 1, current, current + 1, '…', total]
})

function onSelect(target: number) {
  if (props.loading || target === props.page) return
  if (props.pageCount !== undefined && (target < 1 || target > props.pageCount)) return
  if (target < 1) return
  emit('update:page', target)
}
</script>

<template>
  <Teleport :to="dock ?? 'body'" :disabled="!dock || !teleport">
    <nav
      role="navigation"
      :aria-label="t('common.pagination.label')"
      class="m-overlay pointer-events-auto inline-flex items-center gap-1 rounded-full px-2 py-1.5 shadow-2 select-none"
      :class="{ 'fixed bottom-4 left-1/2 -translate-x-1/2 z-30': !dock && teleport }"
    >
      <span
        v-if="loading"
        class="i-lucide-loader-circle animate-spin w-4 h-4 text-primary ml-1 shrink-0"
      />

      <MIconButton
        icon="i-lucide-chevron-left"
        size="sm"
        :title="t('common.pagination.prev')"
        :disabled="page <= 1 || loading"
        @click="onSelect(page - 1)"
      />

      <template v-if="pageCount !== undefined">
        <template v-for="(item, idx) in items" :key="idx">
          <span v-if="item === '…'" class="text-fg-subtle px-1 text-xs">…</span>
          <button
            v-else
            type="button"
            :disabled="loading"
            class="min-w-8 h-8 px-2 rounded-full text-xs font-semibold flex items-center justify-center transition cursor-pointer disabled:cursor-not-allowed"
            :class="item === page ? 'bg-primary text-primary-on' : 'text-fg-muted hover:bg-sunken'"
            @click="onSelect(item)"
          >
            {{ item }}
          </button>
        </template>
      </template>
      <template v-else>
        <span
          class="bg-primary text-primary-on min-w-8 h-8 px-2 rounded-full text-xs font-semibold flex items-center justify-center"
        >
          {{ page }}
        </span>
      </template>

      <MIconButton
        icon="i-lucide-chevron-right"
        size="sm"
        :title="t('common.pagination.next')"
        :disabled="loading || (pageCount !== undefined ? page >= pageCount : !hasNext)"
        @click="onSelect(page + 1)"
      />
    </nav>
  </Teleport>
</template>

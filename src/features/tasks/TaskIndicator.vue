<script setup lang="ts">
import { computed, ref, watch, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useTasksStore } from './store'
import { MProgress } from '../../ui'

const { t } = useI18n()
const router = useRouter()
const tasksStore = useTasksStore()
function formatSpeed(bps: number): string {
  if (bps <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  let val = bps
  let u = 0
  while (val >= 1024 && u < units.length - 1) {
    val /= 1024
    u++
  }
  return `${val.toFixed(u === 0 ? 0 : 1)} ${units[u]}`
}

const activeCount = computed(() => tasksStore.activeTasksCount)
const aggregateSpeed = computed(() => formatSpeed(tasksStore.aggregateSpeedBps))
const progressPercent = computed(() => Math.round(tasksStore.aggregateProgress * 100))

// Tweened active count & progress percentage (~300ms rAF interpolation)
const displayedCount = ref(activeCount.value)
const displayedProgress = ref(progressPercent.value)

let countRaf: number | null = null
let progressRaf: number | null = null

function tweenValue(
  from: number,
  to: number,
  duration: number,
  onUpdate: (val: number) => void,
): number {
  if (from === to) {
    onUpdate(to)
    return 0
  }
  const start = performance.now()
  function tick(now: number) {
    const elapsed = now - start
    const progress = Math.min(1, elapsed / duration)
    // Ease out cubic
    const ease = 1 - Math.pow(1 - progress, 3)
    const current = from + (to - from) * ease
    onUpdate(Math.round(current))
    if (progress < 1) {
      return requestAnimationFrame(tick)
    }
    return 0
  }
  return requestAnimationFrame(tick)
}

watch(activeCount, (newVal, oldVal) => {
  if (countRaf) cancelAnimationFrame(countRaf)
  countRaf = tweenValue(oldVal ?? newVal, newVal, 300, (val) => {
    displayedCount.value = val
  })
}, { immediate: true })

watch(progressPercent, (newVal, oldVal) => {
  if (progressRaf) cancelAnimationFrame(progressRaf)
  progressRaf = tweenValue(oldVal ?? newVal, newVal, 300, (val) => {
    displayedProgress.value = val
  })
}, { immediate: true })

onUnmounted(() => {
  if (countRaf) cancelAnimationFrame(countRaf)
  if (progressRaf) cancelAnimationFrame(progressRaf)
})

function navigateToTasks() {
  router.push('/tasks')
}
</script>

<template>
  <button
    v-if="activeCount > 0"
    type="button"
    class="flex items-center gap-2 px-2.5 py-1 rounded-full text-xs font-medium bg-primary/10 hover:bg-primary/20 text-primary border border-primary/25 cursor-pointer transition-all duration-150 active:scale-95 shadow-sm"
    @click="navigateToTasks"
  >
    <span class="w-3.5 h-3.5 i-lucide-loader-2 animate-spin" />
    <span>{{ t('tasks.indicator.activeTasks', { count: displayedCount }) }}</span>
    <span class="text-fg-muted font-mono text-[11px]">{{ t('tasks.indicator.speed', { speed: aggregateSpeed }) }}</span>
    <div class="w-12 h-1.5 ml-1">
      <MProgress :value="displayedProgress" :max="100" />
    </div>
  </button>
</template>

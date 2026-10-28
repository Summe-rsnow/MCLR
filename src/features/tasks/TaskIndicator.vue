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

// Tweened active count & progress percentage.
// A single persistent rAF loop eases both values toward their targets instead of
// cancelling/restarting a fresh tween on every store event (download progress fires
// per chunk, so the old restart-per-event tween never settled).
const displayedCount = ref(activeCount.value)
const displayedProgress = ref(Math.round(progressPercent.value))

let rafId: number | null = null
let currentProgress = progressPercent.value
let targetProgress = progressPercent.value
let currentCount = activeCount.value
let targetCount = activeCount.value

const TWEEN_FACTOR = 0.25
const SNAP_EPSILON = 0.5

function stepTween(): void {
  currentProgress += (targetProgress - currentProgress) * TWEEN_FACTOR
  const progressSettled = Math.abs(targetProgress - currentProgress) < SNAP_EPSILON
  if (progressSettled) currentProgress = targetProgress
  displayedProgress.value = Math.round(currentProgress)

  currentCount += (targetCount - currentCount) * TWEEN_FACTOR
  const countSettled = Math.abs(targetCount - currentCount) < SNAP_EPSILON
  if (countSettled) currentCount = targetCount
  displayedCount.value = Math.round(currentCount)

  if (progressSettled && countSettled) {
    rafId = null
    return
  }
  rafId = requestAnimationFrame(stepTween)
}

function ensureLoop(): void {
  if (rafId === null) rafId = requestAnimationFrame(stepTween)
}

watch(activeCount, (newVal) => {
  targetCount = newVal
  ensureLoop()
})

watch(progressPercent, (newVal) => {
  targetProgress = newVal
  ensureLoop()
})

onUnmounted(() => {
  if (rafId !== null) {
    cancelAnimationFrame(rafId)
    rafId = null
  }
})

function navigateToTasks() {
  if (router?.push) {
    router.push('/logs?tab=tasks')
  }
}
</script>

<template>
  <button
    v-if="activeCount > 0"
    type="button"
    class="flex items-center gap-2 px-2.5 py-1 rounded-full text-xs font-medium bg-primary/10 hover:bg-primary/20 text-primary border border-primary/25 cursor-pointer transition-all duration-150 active:scale-95 shadow-sm"
    @click="navigateToTasks"
  >
    <span class="w-3.5 h-3.5 i-lucide-loader-circle animate-spin" />
    <span>{{ t('tasks.indicator.activeTasks', { count: displayedCount }) }}</span>
    <span class="text-fg-muted font-mono text-[11px]">{{
      t('tasks.indicator.speed', { speed: aggregateSpeed })
    }}</span>
    <div class="w-12 h-1.5 ml-1">
      <MProgress :value="displayedProgress" :max="100" />
    </div>
  </button>
</template>

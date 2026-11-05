<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { MDialog, MButton, MBadge } from '../../../ui'
import { useLaunchStore } from '../store'

const { t, te } = useI18n()
const launchStore = useLaunchStore()

const isOpen = computed({
  get: () => launchStore.activeMilestone !== null,
  set: (val: boolean) => {
    if (!val) {
      launchStore.dismissMilestone(false)
    }
  },
})

const milestoneKey = computed(() => String(launchStore.activeMilestone || 10))

const title = computed(() => {
  const key = `launch.milestones.${milestoneKey.value}.title`
  return te(key) ? t(key) : ''
})

const body = computed(() => {
  const key = `launch.milestones.${milestoneKey.value}.body`
  return te(key) ? t(key) : ''
})

const confirmText = computed(() => {
  const key = `launch.milestones.${milestoneKey.value}.confirm`
  return te(key) ? t(key) : t('common.confirm')
})

const cancelText = computed(() => {
  const key = `launch.milestones.${milestoneKey.value}.cancel`
  return te(key) ? t(key) : t('common.cancel')
})

function onConfirm() {
  launchStore.dismissMilestone(true)
}

function onCancel() {
  launchStore.dismissMilestone(false)
}
</script>

<template>
  <MDialog :open="isOpen" :title="title" @update:open="!$event && onCancel()">
    <div class="space-y-4 py-1" data-testid="sponsor-milestone-dialog">
      <div class="flex items-start gap-3.5 p-3.5 rounded-xl bg-sunken/50 border border-line">
        <div
          class="w-10 h-10 rounded-xl bg-rose-500/10 text-rose-500 flex items-center justify-center shrink-0"
        >
          <span class="i-lucide-heart-handshake w-5 h-5" />
        </div>
        <div class="min-w-0 flex-1 space-y-1">
          <div class="flex items-center gap-2">
            <span class="text-sm font-semibold text-fg">
              {{ title }}
            </span>
            <MBadge variant="primary" size="sm"> #{{ milestoneKey }} </MBadge>
          </div>
          <p class="text-xs text-fg-muted leading-relaxed select-text">
            {{ body }}
          </p>
        </div>
      </div>
    </div>

    <template #footer>
      <div class="flex items-center justify-end gap-2.5">
        <MButton variant="outline" size="sm" @click="onCancel">
          {{ cancelText }}
        </MButton>
        <MButton variant="primary" size="sm" icon="i-lucide-heart" @click="onConfirm">
          {{ confirmText }}
        </MButton>
      </div>
    </template>
  </MDialog>
</template>

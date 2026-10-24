<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { useCrashStore } from './store'
import { MPageHeader, MButton } from '../../ui'
import CrashDialog from './CrashDialog.vue'

const { t } = useI18n()
const route = useRoute()
const crashStore = useCrashStore()

onMounted(async () => {
  const instanceId = (route.query.instanceId as string) || crashStore.currentInstanceId || 'default'
  if (!crashStore.currentResult) {
    await crashStore.analyzeInstance(instanceId, null, true)
  } else {
    crashStore.openDialog()
  }
})
</script>

<template>
  <div class="m-page">
    <MPageHeader
      :title="t('crash.dialog.title')"
      :description="t('crash.dialog.subtitle')"
      icon="i-lucide-alert-triangle"
    >
      <template #actions>
        <MButton
          variant="primary"
          size="sm"
          icon="i-lucide-activity"
          @click="crashStore.openDialog()"
        >
          {{ t('crash.dialog.title') }}
        </MButton>
      </template>
    </MPageHeader>
    <CrashDialog />
  </div>
</template>

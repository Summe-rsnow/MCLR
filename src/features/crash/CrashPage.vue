<script setup lang="ts">
import { onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useCrashStore } from './store'
import CrashDialog from './CrashDialog.vue'

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
  <div class="p-6">
    <CrashDialog />
  </div>
</template>

<script setup lang="ts">
import type { InstanceInfo } from '../../../ipc/client'
import InstallConfigurator from '../../download/InstallConfigurator.vue'

defineProps<{
  instance: InstanceInfo
}>()
</script>

<template>
  <div class="p-6 max-w-4xl mx-auto">
    <InstallConfigurator
      mode="modify"
      :mc-version="instance.mcVersion"
      :instance="instance"
    />
  </div>
</template>

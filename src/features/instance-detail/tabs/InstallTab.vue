<script setup lang="ts">
import type { InstanceInfo } from '../../../ipc/client'
import InstallConfigurator from '../../download/InstallConfigurator.vue'

defineProps<{
  instance: InstanceInfo
}>()
</script>

<template>
  <div class="w-full space-y-6">
    <InstallConfigurator mode="modify" :mc-version="instance.mcVersion" :instance="instance" />
  </div>
</template>

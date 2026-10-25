<script setup lang="ts">
import ResourceList from '../../resources/ResourceList.vue'

defineProps<{
  instanceId: string
}>()
</script>

<template>
  <div class="h-full">
    <ResourceList :instance-id="instanceId" kind="resourcePack" />
  </div>
</template>

<script setup lang="ts">
import ModListPanel from '../../mods/ModListPanel.vue'

defineProps<{
  instanceId: string
}>()
</script>

<template>
  <div class="h-full">
    <ModListPanel :instance-id="instanceId" />
  </div>
</template>

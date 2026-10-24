<script setup lang="ts">
import { computed } from 'vue'
import { brandIconMap, type BrandName } from './icons'

const props = defineProps<{
  name: BrandName
}>()

const iconClass = computed(() => brandIconMap[props.name] ?? 'i-simple-icons-minecraft')
</script>

<template>
  <span :class="iconClass" class="inline-block shrink-0" aria-hidden="true" />
</template>

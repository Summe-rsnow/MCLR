<script setup lang="ts">
import { computed } from 'vue'
import { Tabs } from '@miuui/vue'
import type { TabsItem } from '@miuui/vue'

const props = defineProps<{
  modelValue: string
  tabs: Array<{ key: string; label: string; icon?: string }>
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
}>()

const tabMap = computed(() => {
  const m = new Map<string, { key: string; label: string; icon?: string }>()
  for (const t of props.tabs) m.set(t.key, t)
  return m
})

const miuItems = computed<TabsItem[]>(() =>
  props.tabs.map((t) => ({ value: t.key, label: t.label })),
)
</script>

<template>
  <Tabs
    :model-value="modelValue"
    :items="miuItems"
    variant="underline"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <template #trigger="{ item }">
      <span
        v-if="tabMap.get(item.value)?.icon"
        :class="tabMap.get(item.value)?.icon"
        class="w-4 h-4 shrink-0"
      />
      <span>{{ item.label }}</span>
    </template>
  </Tabs>
</template>

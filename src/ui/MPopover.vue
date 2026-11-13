<script setup lang="ts">
import { computed } from 'vue'
import { Popover } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    placement?: 'bottom-start' | 'bottom-end' | 'top-start' | 'top-end'
  }>(),
  {
    placement: 'bottom-start',
  },
)

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
}>()

const side = computed<'top' | 'bottom'>(() =>
  props.placement.startsWith('top') ? 'top' : 'bottom',
)
const align = computed<'start' | 'end'>(() => (props.placement.endsWith('end') ? 'end' : 'start'))
</script>

<template>
  <Popover
    :open="open"
    :side="side"
    :align="align"
    :class-names="{ content: 'm-overlay m-refract select-none' }"
    @update:open="emit('update:open', $event)"
  >
    <template #trigger>
      <span class="inline-flex">
        <slot name="trigger" />
      </span>
    </template>
    <div data-testid="m-popover">
      <slot />
    </div>
  </Popover>
</template>

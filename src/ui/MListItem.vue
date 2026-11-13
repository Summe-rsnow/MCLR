<script setup lang="ts">
import { ListItem } from '@miuui/vue'

withDefaults(
  defineProps<{
    title?: string
    subtitle?: string
    active?: boolean
    clickable?: boolean
    disabled?: boolean
    dense?: boolean
  }>(),
  {
    title: undefined,
    subtitle: undefined,
    active: false,
    clickable: true,
    disabled: false,
    dense: false,
  },
)

const emit = defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()
</script>

<template>
  <ListItem
    :title="title"
    :subtitle="subtitle"
    :active="active"
    :clickable="clickable"
    :disabled="disabled"
    :dense="dense"
    :class-names="{
      root: [
        dense ? 'px-2.5 py-1.5' : 'px-3 py-2.5',
        active ? 'bg-primary-soft text-primary-soft-fg font-medium' : 'text-fg',
      ].join(' '),
    }"
    @click="emit('click', $event)"
  >
    <template v-if="$slots.prefix" #prefix>
      <slot name="prefix" />
    </template>
    <template v-if="$slots.title" #title>
      <slot name="title" />
    </template>
    <template v-if="$slots.default" #default>
      <slot />
    </template>
    <template v-if="$slots.badges" #badges>
      <slot name="badges" />
    </template>
    <template v-if="$slots.subtitle" #subtitle>
      <slot name="subtitle" />
    </template>
    <template v-if="$slots.actions || $slots.suffix" #actions>
      <slot name="actions">
        <slot name="suffix" />
      </slot>
    </template>
  </ListItem>
</template>

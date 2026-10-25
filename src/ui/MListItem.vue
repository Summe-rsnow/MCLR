<script setup lang="ts">
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

defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()
</script>

<template>
  <div
    class="flex items-center justify-between gap-3 rounded-lg transition-colors select-none"
    :class="[
      dense ? 'px-2.5 py-1.5' : 'px-3 py-2.5',
      clickable && !disabled ? 'cursor-pointer hover:bg-sunken' : '',
      active ? 'bg-primary-soft text-primary-soft-fg font-medium' : 'text-fg',
      disabled ? 'opacity-45 cursor-not-allowed' : '',
    ]"
    @click="!disabled && $emit('click', $event)"
  >
    <div class="flex items-center gap-3 min-w-0 flex-1">
      <slot name="prefix" />
      <div class="flex flex-col min-w-0 flex-1">
        <div class="flex items-center gap-2 min-w-0">
          <slot name="title">
            <span v-if="title" class="text-sm truncate">{{ title }}</span>
            <slot v-else />
          </slot>
          <slot name="badges" />
        </div>
        <slot name="subtitle">
          <span v-if="subtitle" class="text-xs text-fg-muted truncate">{{ subtitle }}</span>
        </slot>
      </div>
    </div>
    <div v-if="$slots.actions || $slots.suffix" class="flex items-center gap-2 shrink-0">
      <slot name="actions">
        <slot name="suffix" />
      </slot>
    </div>
  </div>
</template>

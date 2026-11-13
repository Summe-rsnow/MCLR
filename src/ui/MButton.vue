<script setup lang="ts">
import { computed } from 'vue'
import { Button } from '@miuui/vue'
import type { ButtonVariant } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    variant?: 'primary' | 'tonal' | 'outline' | 'text' | 'danger'
    size?: 'xs' | 'sm' | 'md' | 'lg'
    icon?: string
    iconRight?: string
    block?: boolean
    loading?: boolean
    disabled?: boolean
    type?: 'button' | 'submit' | 'reset'
  }>(),
  {
    variant: 'primary',
    size: 'md',
    icon: undefined,
    iconRight: undefined,
    block: false,
    loading: false,
    disabled: false,
    type: 'button',
  },
)

const emit = defineEmits<{
  (e: 'click', evt: MouseEvent): void
}>()

const miuVariant = computed<ButtonVariant>(() => {
  switch (props.variant) {
    case 'primary':
      return 'solid'
    case 'text':
      return 'ghost'
    case 'tonal':
      return 'tonal'
    case 'outline':
      return 'outline'
    case 'danger':
      return 'danger'
    default:
      return 'solid'
  }
})

const iconSizeClass = computed(() => {
  switch (props.size) {
    case 'xs':
      return 'w-3.5 h-3.5'
    case 'lg':
      return 'w-5 h-5'
    case 'sm':
    case 'md':
    default:
      return 'w-4 h-4'
  }
})
</script>

<template>
  <Button
    :variant="miuVariant"
    :size="size"
    :block="block"
    :loading="loading"
    :disabled="disabled"
    :type="type"
    radius="md"
    :class-names="{
      root: variant === 'primary' ? 'm-btn-primary' : undefined,
      spinner: 'animate-spin',
    }"
    @click="emit('click', $event)"
  >
    <template v-if="$slots.icon || icon" #icon>
      <slot name="icon">
        <span v-if="icon" :class="[icon, iconSizeClass]" class="shrink-0" />
      </slot>
    </template>
    <slot />
    <span v-if="iconRight" :class="[iconRight, iconSizeClass]" class="shrink-0" />
  </Button>
</template>

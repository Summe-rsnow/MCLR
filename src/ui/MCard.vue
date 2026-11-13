<script setup lang="ts">
import { computed, useAttrs } from 'vue'
import { Card } from '@miuui/vue'
import MSkeleton from './MSkeleton.vue'

defineOptions({ inheritAttrs: false })

const props = withDefaults(
  defineProps<{
    variant?: 'raised' | 'well' | 'ghost' | 'tinted'
    padding?: 'none' | 'sm' | 'md' | 'lg'
    hoverable?: boolean
    loading?: boolean
    skeletonLines?: number
  }>(),
  {
    variant: 'raised',
    padding: 'md',
    hoverable: false,
    loading: false,
    skeletonLines: 3,
  },
)

const attrs = useAttrs()

const BODY_LAYOUT_RE =
  /^(?:[a-z0-9-]+:)*(?:flex|inline-flex|flex-col|flex-row|flex-col-reverse|flex-row-reverse|flex-wrap|flex-nowrap|items-|justify-|content-|place-|gap-|space-[xy]-|grid|inline-grid|grid-cols-|text-(?:left|center|right|justify)|divide-)/

const splitClasses = computed(() => {
  const raw =
    typeof attrs.class === 'string'
      ? attrs.class
      : Array.isArray(attrs.class)
        ? attrs.class.join(' ')
        : ''
  const tokens = raw.split(/\s+/).filter(Boolean)
  const root: string[] = []
  const body: string[] = []
  for (const token of tokens) {
    if (BODY_LAYOUT_RE.test(token)) {
      body.push(token)
    } else {
      root.push(token)
    }
  }
  if (tokens.includes('h-full') && body.some((t) => /(?:^|:)(?:flex|grid)$/.test(t))) {
    body.push('h-full')
  }
  return { root: root.join(' '), body: body.join(' ') }
})

const passthroughAttrs = computed(() => {
  const rest: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(attrs)) {
    if (k !== 'class') rest[k] = v
  }
  return rest
})

const miuVariant = computed(() => {
  if (props.variant === 'ghost') return 'ghost'
  if (props.variant === 'well' || props.variant === 'tinted') return 'outline'
  return 'surface'
})

const rootVariantClass = computed(() => {
  switch (props.variant) {
    case 'well':
      return 'm-well'
    case 'ghost':
      return 'bg-transparent border-transparent shadow-none'
    case 'tinted':
      return 'bg-primary-soft text-fg border-primary/15 shadow-none'
    case 'raised':
    default:
      return 'm-card'
  }
})
</script>

<template>
  <Card
    v-bind="passthroughAttrs"
    :variant="miuVariant"
    :padding="padding"
    :interactive="hoverable"
    radius="lg"
    :class-names="{
      root: ['m-card-flow', rootVariantClass, hoverable ? 'm-card-hover' : '', splitClasses.root]
        .filter(Boolean)
        .join(' '),
      body: ['flex-1 min-h-0', splitClasses.body].filter(Boolean).join(' '),
    }"
  >
    <div v-if="loading" class="space-y-3">
      <MSkeleton height="1.25rem" width="40%" rounded="lg" />
      <MSkeleton
        v-for="i in Math.max(0, skeletonLines - 1)"
        :key="i"
        height="0.875rem"
        :width="i === skeletonLines - 1 ? '60%' : '100%'"
        rounded="md"
      />
    </div>
    <slot v-else />
  </Card>
</template>

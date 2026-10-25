<script setup lang="ts">
import type { JarInJarNode } from '../../ipc/client'

defineProps<{
  node: JarInJarNode
  depth?: number
}>()

function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + ' MB'
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB'
  return bytes + ' B'
}
</script>

<template>
  <div class="space-y-1">
    <div
      class="flex items-center justify-between p-2 rounded-lg bg-sunken/30 hover:bg-sunken/50 transition border border-line text-xs"
      :style="{ marginLeft: `${(depth || 0) * 16}px` }"
    >
      <div class="flex items-center space-x-2 min-w-0">
        <span class="i-lucide-package text-primary shrink-0 w-3.5 h-3.5" />
        <span class="font-medium text-fg truncate" :title="node.name">
          {{ node.name }}
        </span>
        <span class="text-fg-muted">v{{ node.version }}</span>
      </div>
      <div class="flex items-center space-x-2 shrink-0 text-fg-muted">
        <span class="font-mono text-[11px]">{{ node.id }}</span>
        <span>•</span>
        <span>{{ formatSize(node.size) }}</span>
      </div>
    </div>

    <!-- Recursive children -->
    <template v-if="node.children && node.children.length > 0">
      <JarInJarItem
        v-for="child in node.children"
        :key="child.id + child.version"
        :node="child"
        :depth="(depth || 0) + 1"
      />
    </template>
  </div>
</template>

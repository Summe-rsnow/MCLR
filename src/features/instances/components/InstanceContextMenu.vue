<script setup lang="ts">
/**
 * Right-click menu for one instance.
 *
 * Owns the item list (labels, icons, star/hide state) so every page that can
 * show an instance card renders the *same* menu instead of re-deriving it. The
 * page supplies the instance under the cursor and handles the emitted action.
 */
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import type { InstanceInfo } from '../../../ipc/client'
import { MContextMenu, type ContextMenuItem } from '../../../ui'

const props = defineProps<{
  instance: InstanceInfo | null
  x: number
  y: number
  /** Openers that need a confirmation dialog can opt out of the destructive entries. */
  compact?: boolean
  /** Shift+right-click targets the permanent-delete entry. */
  permanent?: boolean
}>()

const open = defineModel<boolean>('open', { default: false })

const emit = defineEmits<{
  action: [key: string, instance: InstanceInfo]
}>()

const { t } = useI18n()

const items = computed<ContextMenuItem[]>(() => {
  const inst = props.instance
  if (!inst) return []

  const isStarred = Boolean(inst.config?.starred || inst.config?.pinned)
  const isHidden = inst.card === 'hidden' || inst.config?.cardType === 'hidden'

  const base: ContextMenuItem[] = [
    { key: 'manage', label: t('instances.actions.manage'), icon: 'i-lucide-settings-2' },
    { key: 'openFolder', label: t('instances.actions.openFolder'), icon: 'i-lucide-folder-open' },
    {
      key: 'star',
      label: isStarred ? t('instances.actions.unstar') : t('instances.actions.star'),
      icon: isStarred ? 'i-lucide-star-off' : 'i-lucide-star',
    },
  ]

  if (props.compact) return base

  return [
    ...base,
    { key: 'rename', label: t('instances.actions.rename'), icon: 'i-lucide-square-pen' },
    { key: 'duplicate', label: t('instances.actions.duplicate'), icon: 'i-lucide-copy' },
    {
      key: 'hide',
      label: isHidden ? t('instances.actions.unhide') : t('instances.actions.hide'),
      icon: isHidden ? 'i-lucide-eye' : 'i-lucide-eye-off',
    },
    {
      key: props.permanent ? 'deletePermanent' : 'delete',
      label: props.permanent
        ? t('instances.actions.deletePermanent')
        : t('instances.actions.delete'),
      icon: 'i-lucide-trash',
      danger: true,
    },
  ]
})

function onSelect(key: string) {
  if (!props.instance) return
  emit('action', key, props.instance)
}
</script>

<template>
  <MContextMenu v-model:open="open" :items="items" :x="x" :y="y" @select="onSelect" />
</template>

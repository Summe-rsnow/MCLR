<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { InstanceInfo } from '../../../ipc/client'
import { MDialog, MInput, MButton } from '../../../ui'

const props = defineProps<{
  renameOpen: boolean
  duplicateOpen: boolean
  instance: InstanceInfo | null
}>()

const emit = defineEmits<{
  (e: 'update:renameOpen', val: boolean): void
  (e: 'update:duplicateOpen', val: boolean): void
  (e: 'rename', name: string): void
  (e: 'duplicate', name: string): void
}>()

const { t } = useI18n()
const instanceNewName = ref('')
const instanceNameError = ref('')
const invalidNameRegex = /[\\/:*?"<>|!;]/

watch(
  () => props.renameOpen,
  (open) => {
    if (open && props.instance) {
      instanceNewName.value = props.instance.name
      instanceNameError.value = ''
    }
  },
)

watch(
  () => props.duplicateOpen,
  (open) => {
    if (open && props.instance) {
      instanceNewName.value = `${props.instance.name} (Copy)`
      instanceNameError.value = ''
    }
  },
)

function handleRename() {
  const name = instanceNewName.value.trim()
  if (!name || invalidNameRegex.test(name) || name.endsWith('.') || name.length > 70) {
    instanceNameError.value = t('instances.dialog.nameError')
    return
  }
  emit('rename', name)
  emit('update:renameOpen', false)
}

function handleDuplicate() {
  const name = instanceNewName.value.trim()
  if (!name || invalidNameRegex.test(name) || name.endsWith('.') || name.length > 70) {
    instanceNameError.value = t('instances.dialog.nameError')
    return
  }
  emit('duplicate', name)
  emit('update:duplicateOpen', false)
}
</script>

<template>
  <div>
    <!-- Rename Dialog -->
    <MDialog
      :open="renameOpen"
      :title="t('instances.dialog.renameTitle')"
      @update:open="emit('update:renameOpen', $event)"
    >
      <div class="space-y-4 py-2">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          @keyup.enter="handleRename"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="tonal" @click="emit('update:renameOpen', false)">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" @click="handleRename">
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>

    <!-- Duplicate Dialog -->
    <MDialog
      :open="duplicateOpen"
      :title="t('instances.dialog.duplicateTitle')"
      @update:open="emit('update:duplicateOpen', $event)"
    >
      <div class="space-y-4 py-2">
        <MInput
          v-model="instanceNewName"
          :label="t('instances.dialog.nameLabel')"
          :placeholder="t('instances.dialog.namePlaceholder')"
          :error="instanceNameError"
          @keyup.enter="handleDuplicate"
        />
      </div>
      <template #footer>
        <div class="flex justify-end gap-2">
          <MButton variant="tonal" @click="emit('update:duplicateOpen', false)">
            {{ t('common.cancel') }}
          </MButton>
          <MButton variant="primary" @click="handleDuplicate">
            {{ t('common.confirm') }}
          </MButton>
        </div>
      </template>
    </MDialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import type { JarInJarNode, ResourceEntry } from '../../ipc/client'
import { ipcClient } from '../../ipc/client'
import { MDialog, MEmpty, MSkeleton, MButton } from '../../ui'
import JarInJarItem from './JarInJarItem.vue'

const props = defineProps<{
  modelValue: boolean
  instanceId: string
  mod: ResourceEntry | null
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const { t } = useI18n()
const loading = ref(false)
const nodes = ref<JarInJarNode[]>([])

watch(
  () => props.modelValue,
  async (open) => {
    if (open && props.mod) {
      loading.value = true
      nodes.value = []
      try {
        nodes.value = await ipcClient.localModJarInJar(
          props.instanceId,
          props.mod.fileName,
        )
      } catch {
        nodes.value = []
      } finally {
        loading.value = false
      }
    }
  },
)

function onClose() {
  emit('update:modelValue', false)
}
</script>

<template>
  <MDialog
    :open="modelValue"
    :title="t('resources.jarInJarDrawer.title')"
    class="max-w-xl"
    @update:open="emit('update:modelValue', $event)"
    @cancel="onClose"
  >
    <div class="space-y-4">
      <div v-if="mod" class="text-xs text-fg-muted">
        {{ t('resources.jarInJarDrawer.subtitle', { name: mod.title || mod.fileName }) }}
      </div>

      <div v-if="loading" class="space-y-2 py-4">
        <MSkeleton class="h-10 rounded-lg" />
        <MSkeleton class="h-10 rounded-lg" />
        <MSkeleton class="h-10 rounded-lg" />
      </div>

      <div
        v-else-if="nodes.length > 0"
        class="max-h-96 overflow-y-auto space-y-2 pr-1"
      >
        <JarInJarItem
          v-for="item in nodes"
          :key="item.id + item.version"
          :node="item"
        />
      </div>

      <MEmpty
        v-else
        :title="t('resources.jarInJarDrawer.empty')"
        class="py-8"
      />
    </div>

    <template #footer>
      <div class="flex justify-end">
        <MButton variant="tonal" @click="onClose">
          {{ t('resources.actions.cancel') }}
        </MButton>
      </div>
    </template>
  </MDialog>
</template>

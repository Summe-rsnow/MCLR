<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ipcClient } from '../../../ipc/client'
import { useToast } from '../../../ui/useToast'
import { useInstancesStore } from '../../instances/store'
import { MCard, MButton, MInput, MSelect, MHelpTooltip } from '../../../ui'

const { t } = useI18n()
const toast = useToast()
const instancesStore = useInstancesStore()

const shortcutInstanceId = ref('')
const shortcutName = ref('')
const shortcutLoading = ref(false)

const instanceOptions = computed(() => {
  return instancesStore.instances.map((i) => ({
    label: `${i.name} (${i.mcVersion})`,
    value: i.id,
  }))
})

async function createShortcut() {
  if (!shortcutInstanceId.value) {
    toast.warning(t('tools.shortcut.noInstance'))
    return
  }
  shortcutLoading.value = true
  try {
    await ipcClient.toolsCreateShortcut(shortcutInstanceId.value, shortcutName.value.trim() || null)
    toast.success(t('tools.shortcut.created'))
    shortcutName.value = ''
  } catch (err: unknown) {
    toast.danger(err instanceof Error ? err.message : String(err))
  } finally {
    shortcutLoading.value = false
  }
}

onMounted(async () => {
  if (instancesStore.instances.length === 0) {
    await instancesStore.fetchInstances()
  }
  if (instancesStore.instances.length > 0 && !shortcutInstanceId.value) {
    shortcutInstanceId.value = instancesStore.instances[0].id
  }
})
</script>

<template>
  <MCard hoverable class="p-5 flex flex-col space-y-4 m-stagger" :style="{ '--i': 3 }">
    <div>
      <div class="flex items-center gap-3">
        <div
          class="w-10 h-10 rounded-xl bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
        >
          <span class="i-lucide-external-link w-5 h-5 inline-block" />
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center gap-1.5">
            <h2 class="text-base font-semibold text-fg">{{ t('tools.shortcut.title') }}</h2>
            <MHelpTooltip :text="t('tools.shortcut.desc')" />
          </div>
          <p class="text-xs text-fg-muted mt-0.5 truncate">
            {{ t('tools.shortcut.desc') }}
          </p>
        </div>
      </div>
      <div class="mt-4 space-y-3">
        <div>
          <label class="text-xs text-fg-muted block mb-1 font-medium">{{
            t('tools.shortcut.selectInstance')
          }}</label>
          <MSelect v-model="shortcutInstanceId" :options="instanceOptions" class="w-full" />
        </div>

        <div>
          <label class="text-xs text-fg-muted block mb-1 font-medium">{{
            t('tools.shortcut.nameLabel')
          }}</label>
          <MInput
            v-model="shortcutName"
            :placeholder="t('tools.shortcut.namePlaceholder')"
            class="w-full"
          />
        </div>
      </div>
    </div>
    <div class="pt-2 flex justify-end">
      <MButton
        variant="primary"
        icon="i-lucide-monitor"
        :loading="shortcutLoading"
        data-testid="create-shortcut-btn"
        @click="createShortcut"
      >
        {{ t('tools.shortcut.create') }}
      </MButton>
    </div>
  </MCard>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ipcClient } from '../../../ipc/client'
import { MButton, MSkeleton } from '../../../ui'
import ResourceList from '../../resources/ResourceList.vue'

const props = defineProps<{
  instanceId: string
}>()

const router = useRouter()
const { t } = useI18n()

const checking = ref(true)
const folderMissing = ref(false)

async function checkFolder() {
  checking.value = true
  folderMissing.value = false
  try {
    await ipcClient.resourcesList(props.instanceId, 'schematic')
  } catch (err: unknown) {
    const errStr = String(err)
    if (errStr.includes('fileNotFound') || errStr.includes('schematics')) {
      folderMissing.value = true
    }
  } finally {
    checking.value = false
  }
}

function handleDownloadLitematica() {
  router.push(`/comp/mod?instance=${props.instanceId}&q=litematica`)
}

function handleSelectInstance() {
  router.push('/instances')
}

onMounted(() => {
  checkFolder()
})

watch(
  () => props.instanceId,
  () => {
    checkFolder()
  },
)
</script>

<template>
  <div class="h-full">
    <div v-if="checking" class="space-y-3 py-6">
      <MSkeleton class="h-16 rounded-xl" />
      <MSkeleton class="h-16 rounded-xl" />
    </div>

    <div
      v-else-if="folderMissing"
      class="flex flex-col items-center justify-center text-center py-16 px-4 space-y-4"
    >
      <div
        class="w-16 h-16 rounded-2xl bg-sunken flex items-center justify-center text-fg-muted mb-2 border border-line"
      >
        <span class="i-lucide-file-code w-8 h-8 text-primary" />
      </div>
      <h3 class="text-base font-semibold text-fg">
        {{ t('resources.schematic.disabledTitle') }}
      </h3>
      <p class="text-xs text-fg-muted leading-relaxed">
        {{ t('resources.schematic.disabledDesc') }}
      </p>
      <div class="flex items-center space-x-3 pt-2">
        <MButton variant="primary" @click="handleDownloadLitematica">
          <span class="i-lucide-download mr-1.5 w-4 h-4" />
          {{ t('resources.schematic.downloadLitematica') }}
        </MButton>
        <MButton variant="tonal" @click="handleSelectInstance">
          {{ t('resources.schematic.selectInstance') }}
        </MButton>
      </div>
    </div>

    <ResourceList v-else :instance-id="instanceId" kind="schematic" />
  </div>
</template>

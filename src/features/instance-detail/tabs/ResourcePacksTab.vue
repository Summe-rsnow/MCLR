<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import InstanceFilesTab from './InstanceFilesTab.vue'

defineProps<{
  instanceId: string
}>()

const { t } = useI18n()
</script>

<template>
  <InstanceFilesTab
    :instance-id="instanceId"
    folder-kind="resourcepacks"
    :title="t('instanceDetail.tabs.resourcepacks')"
  />
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { MCard, MButton, MEmpty } from '../../../ui'

const { t } = useI18n()
const router = useRouter()
</script>

<template>
  <MCard padding="lg" class="py-16 flex flex-col items-center justify-center">
    <MEmpty
      icon="i-lucide-ban"
      :title="t('instanceDetail.modsDisabled.title')"
      :description="t('instanceDetail.modsDisabled.description')"
    >
      <div class="flex flex-wrap items-center justify-center gap-3 mt-4">
        <MButton
          variant="primary"
          size="sm"
          icon="i-lucide-download"
          @click="router.push('/download')"
        >
          {{ t('instanceDetail.modsDisabled.downloadModable') }}
        </MButton>
        <MButton
          variant="tonal"
          size="sm"
          icon="i-lucide-boxes"
          @click="router.push('/instances')"
        >
          {{ t('instanceDetail.modsDisabled.chooseOther') }}
        </MButton>
      </div>
    </MEmpty>
  </MCard>
</template>

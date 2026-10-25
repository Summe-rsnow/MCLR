<script setup lang="ts">
import { ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { MDialog, MInput, MSelect, MButton } from '../../../ui'

const props = defineProps<{
  open: boolean
  editing: boolean
  initialName?: string
  initialIp?: string
  initialAcceptTextures?: string
  loading?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:open', val: boolean): void
  (e: 'save', data: { name: string; ip: string; acceptTextures: string }): void
}>()

const { t } = useI18n()

const form = ref({
  name: '',
  ip: '',
  acceptTextures: '0',
})

const acceptTexturesOptions = [
  { label: t('instanceDetail.servers.acceptTexturesPrompt'), value: '0' },
  { label: t('instanceDetail.servers.acceptTexturesEnabled'), value: '1' },
  { label: t('instanceDetail.servers.acceptTexturesDisabled'), value: '2' },
]

watch(
  () => props.open,
  (val) => {
    if (val) {
      form.value = {
        name: props.initialName ?? '',
        ip: props.initialIp ?? '',
        acceptTextures: props.initialAcceptTextures ?? '0',
      }
    }
  },
  { immediate: true },
)

function onSave() {
  const name = form.value.name.trim()
  const ip = form.value.ip.trim()
  if (!name || !ip) return
  emit('save', {
    name,
    ip,
    acceptTextures: form.value.acceptTextures,
  })
}
</script>

<template>
  <MDialog
    :open="open"
    :title="editing ? t('instanceDetail.servers.editTitle') : t('instanceDetail.servers.addTitle')"
    @update:open="emit('update:open', $event)"
  >
    <div class="py-2 space-y-4 min-w-80">
      <MInput
        v-model="form.name"
        :label="t('instanceDetail.servers.name')"
        :placeholder="t('instanceDetail.servers.namePlaceholder')"
        autofocus
      />

      <MInput
        v-model="form.ip"
        :label="t('instanceDetail.servers.ip')"
        :placeholder="t('instanceDetail.servers.ipPlaceholder')"
        @keydown.enter="onSave"
      />

      <div class="space-y-1.5">
        <label class="text-sm font-semibold text-fg">
          {{ t('instanceDetail.servers.acceptTextures') }}
        </label>
        <MSelect v-model="form.acceptTextures" :options="acceptTexturesOptions" />
      </div>
    </div>
    <template #footer>
      <div class="flex justify-end gap-2">
        <MButton variant="text" @click="emit('update:open', false)">
          {{ t('common.cancel') }}
        </MButton>
        <MButton variant="primary" :loading="loading" @click="onSave">
          {{ t('common.confirm') }}
        </MButton>
      </div>
    </template>
  </MDialog>
</template>

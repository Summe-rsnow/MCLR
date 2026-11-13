<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { Pagination } from '@miuui/vue'

const props = withDefaults(
  defineProps<{
    page: number
    pageCount?: number
    hasNext?: boolean
    loading?: boolean
    teleport?: boolean
  }>(),
  {
    pageCount: undefined,
    hasNext: true,
    loading: false,
    teleport: true,
  },
)

const emit = defineEmits<{
  (e: 'update:page', page: number): void
}>()

const { t } = useI18n()
const dock = ref<HTMLElement | null>(null)

onMounted(() => {
  dock.value = document.getElementById('m-float-dock')
})

const total = computed(() =>
  props.pageCount !== undefined ? Math.max(1, props.pageCount) * 20 : undefined,
)
</script>

<template>
  <Teleport v-if="teleport && dock" :to="dock">
    <div class="pointer-events-auto pb-5 pt-2 flex justify-center">
      <Pagination
        mode="numeric"
        :page="page"
        :page-size="20"
        :total="total"
        :has-next="hasNext"
        :loading="loading"
        :show-page-size="false"
        :show-quick-jumper="false"
        :aria-label="t('common.pagination.label')"
        :class-names="{ root: 'm-overlay m-refract' }"
        @update:page="emit('update:page', $event)"
      />
    </div>
  </Teleport>
  <Pagination
    v-else
    mode="numeric"
    :page="page"
    :page-size="20"
    :total="total"
    :has-next="hasNext"
    :loading="loading"
    :show-page-size="false"
    :show-quick-jumper="false"
    :aria-label="t('common.pagination.label')"
    :class-names="{ root: 'm-overlay m-refract' }"
    @update:page="emit('update:page', $event)"
  />
</template>

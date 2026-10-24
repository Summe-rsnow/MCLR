<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { useThemeStore } from '../../stores/theme'
import {
  MButton,
  MIconButton,
  MCard,
  MInput,
  MTextarea,
  MSelect,
  MSwitch,
  MCheckbox,
  MRadioGroup,
  MSlider,
  MSegmented,
  MTabs,
  MList,
  MListItem,
  MDialog,
  MTooltip,
  MProgress,
  MScroll,
  MContextMenu,
  MColorPicker,
  MEmpty,
  MSkeleton,
  MBadge,
  MAvatar,
  useDialog,
  useToast,
  type ContextMenuItem,
} from '../../ui'

const { t } = useI18n()
const themeStore = useThemeStore()
const dialog = useDialog()
const toast = useToast()

// Theme controls
const modeOptions = [
  { label: t('devUi.modeLight'), value: 'light' },
  { label: t('devUi.modeDark'), value: 'dark' },
  { label: t('devUi.modeSystem'), value: 'system' },
]

const materialOptions = [
  { label: t('devUi.materialFlat'), value: 'flat' },
  { label: t('devUi.materialGlass'), value: 'glass' },
]

// Component state
const inputText = ref('')
const textareaText = ref('')
const selectVal = ref('opt1')
const selectOptions = [
  { label: 'Option 1', value: 'opt1' },
  { label: 'Option 2', value: 'opt2' },
  { label: 'Option 3', value: 'opt3' },
]

const switchVal = ref(true)
const checkboxVal = ref(true)
const radioVal = ref('r1')
const radioOptions = [
  { label: 'Radio A', value: 'r1' },
  { label: 'Radio B', value: 'r2' },
]

const sliderVal = ref(45)
const segmentedVal = ref('tab1')
const segmentedOptions = [
  { label: 'Alpha', value: 'tab1', icon: 'i-lucide-box' },
  { label: 'Beta', value: 'tab2', icon: 'i-lucide-cpu' },
  { label: 'Gamma', value: 'tab3', icon: 'i-lucide-globe' },
]

const activeTab = ref('general')
const tabsList = [
  { key: 'general', label: 'General', icon: 'i-lucide-settings' },
  { key: 'advanced', label: 'Advanced', icon: 'i-lucide-sliders' },
  { key: 'about', label: 'About', icon: 'i-lucide-info' },
]

const progressVal = ref(0.68)
const dialogVisible = ref(false)

// Context Menu
const contextMenuOpen = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuItems: ContextMenuItem[] = [
  { key: 'edit', label: 'Edit', icon: 'i-lucide-edit' },
  { key: 'copy', label: 'Copy', icon: 'i-lucide-copy' },
  { key: 'delete', label: 'Delete', icon: 'i-lucide-trash', danger: true },
]

function openContextMenu(evt: MouseEvent) {
  evt.preventDefault()
  contextMenuX.value = evt.clientX
  contextMenuY.value = evt.clientY
  contextMenuOpen.value = true
}

async function triggerConfirmDialog() {
  const confirmed = await dialog.confirm({
    title: t('devUi.dialogConfirm'),
    content: t('devUi.cardDescription'),
  })
  if (confirmed) {
    toast.success(t('common.confirm'))
  } else {
    toast.info(t('common.cancel'))
  }
}
</script>

<template>
  <div class="max-w-5xl mx-auto space-y-8 pb-12 select-none" @contextmenu="openContextMenu">
    <div>
      <h2 class="text-2xl font-bold tracking-tight text-text">{{ t('devUi.title') }}</h2>
      <p class="text-sm text-textMuted mt-1">{{ t('devUi.themeSettings') }}</p>
    </div>

    <!-- Theme & Material Controllers -->
    <MCard padding="lg" class="space-y-4">
      <h3 class="text-sm font-semibold text-text uppercase tracking-wider">{{ t('devUi.themeSettings') }}</h3>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6 items-start">
        <div class="space-y-2">
          <label class="text-xs font-medium text-textMuted">{{ t('devUi.seedColor') }}</label>
          <MColorPicker
            :model-value="themeStore.seed"
            @update:model-value="themeStore.setSeed($event)"
          />
        </div>

        <div class="space-y-2">
          <label class="text-xs font-medium text-textMuted">{{ t('devUi.themeMode') }}</label>
          <MSegmented
            :model-value="themeStore.mode"
            :options="modeOptions"
            @update:model-value="themeStore.setMode($event as any)"
          />
        </div>

        <div class="space-y-2">
          <label class="text-xs font-medium text-textMuted">{{ t('devUi.materialMode') }}</label>
          <MSegmented
            :model-value="themeStore.material"
            :options="materialOptions"
            @update:model-value="themeStore.setMaterial($event as any)"
          />
        </div>
      </div>
    </MCard>

    <!-- Buttons & Icon Buttons -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.buttons') }}</h3>
      <div class="flex flex-wrap gap-3 items-center">
        <MButton variant="primary">{{ t('common.save') }}</MButton>
        <MButton variant="tonal">{{ t('common.search') }}</MButton>
        <MButton variant="text">{{ t('common.cancel') }}</MButton>
        <MButton variant="danger">{{ t('common.delete') }}</MButton>
        <MButton variant="primary" loading>{{ t('common.loading') }}</MButton>
        <MButton variant="primary" disabled>{{ t('common.save') }}</MButton>

        <div class="h-6 w-px bg-outline/50 mx-2" />

        <MIconButton icon="i-lucide-play" size="sm" />
        <MIconButton icon="i-lucide-settings" size="md" />
        <MIconButton icon="i-lucide-trash" size="lg" />
        <MIconButton icon="i-lucide-refresh-cw" disabled />
      </div>
    </div>

    <!-- Cards (Flat vs Glass showcase) -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.cards') }}</h3>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <MCard>
          <h4 class="font-semibold text-text mb-1">{{ t('devUi.cardTitle') }} (Auto Material)</h4>
          <p class="text-sm text-textMuted">{{ t('devUi.cardDescription') }}</p>
        </MCard>

        <MCard :glass="true">
          <h4 class="font-semibold text-text mb-1">{{ t('devUi.cardTitle') }} (Forced Glass)</h4>
          <p class="text-sm text-textMuted">{{ t('devUi.cardDescription') }}</p>
        </MCard>
      </div>
    </div>

    <!-- Form Inputs & Select -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.inputs') }}</h3>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <MInput v-model="inputText" :placeholder="t('devUi.placeholder')" clearable />
        <MSelect v-model="selectVal" :options="selectOptions" />
        <MInput model-value="Disabled Value" disabled />
      </div>
      <MTextarea v-model="textareaText" :placeholder="t('devUi.placeholder')" :rows="2" />
    </div>

    <!-- Toggles, Checkbox, Radios -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.switches') }}</h3>
      <div class="flex flex-wrap gap-6 items-center">
        <MSwitch v-model="switchVal" label="Switch toggle" />
        <MCheckbox v-model="checkboxVal" label="Checkbox option" />
        <MRadioGroup v-model="radioVal" :options="radioOptions" />
      </div>
    </div>

    <!-- Sliders -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.sliders') }}</h3>
      <div class="max-w-md">
        <MSlider v-model="sliderVal" :min="0" :max="100" />
      </div>
    </div>

    <!-- Navigation: Tabs & Segmented -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.navigation') }}</h3>
      <div class="space-y-4">
        <MSegmented v-model="segmentedVal" :options="segmentedOptions" />
        <MTabs v-model="activeTab" :tabs="tabsList" />
      </div>
    </div>

    <!-- Lists & Scroll -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.lists') }}</h3>
      <MCard padding="none" class="max-w-md overflow-hidden">
        <MScroll max-height="160px">
          <MList>
            <MListItem
              :title="`${t('devUi.itemTitle')} 1`"
              :subtitle="t('devUi.itemSubtitle')"
              active
            >
              <template #prefix>
                <span class="i-lucide-folder w-4 h-4 text-primary" />
              </template>
              <template #suffix>
                <MBadge variant="primary">Active</MBadge>
              </template>
            </MListItem>
            <MListItem
              :title="`${t('devUi.itemTitle')} 2`"
              :subtitle="t('devUi.itemSubtitle')"
            >
              <template #prefix>
                <span class="i-lucide-file w-4 h-4 text-textMuted" />
              </template>
            </MListItem>
            <MListItem
              :title="`${t('devUi.itemTitle')} 3`"
              :subtitle="t('devUi.itemSubtitle')"
            >
              <template #prefix>
                <span class="i-lucide-box w-4 h-4 text-textMuted" />
              </template>
            </MListItem>
          </MList>
        </MScroll>
      </MCard>
    </div>

    <!-- Feedback: Progress, Badges, Skeleton, Avatar -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.feedback') }}</h3>
      <div class="space-y-4">
        <div class="max-w-md space-y-2">
          <MProgress :value="progressVal" show-label />
          <MProgress indeterminate />
        </div>

        <div class="flex flex-wrap gap-2 items-center">
          <MBadge variant="primary">Primary</MBadge>
          <MBadge variant="success">Success</MBadge>
          <MBadge variant="warning">Warning</MBadge>
          <MBadge variant="danger">Danger</MBadge>
          <MBadge variant="neutral">Neutral</MBadge>
        </div>

        <div class="flex items-center gap-4">
          <MAvatar name="Alex" :size="40" />
          <MAvatar name="Steve" :size="40" />
          <div class="space-y-1.5 flex-1 max-w-xs">
            <MSkeleton width="100%" height="0.875rem" />
            <MSkeleton width="60%" height="0.75rem" />
          </div>
        </div>
      </div>
    </div>

    <!-- Overlays: Dialog, Toast, Tooltip -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.overlays') }}</h3>
      <div class="flex flex-wrap gap-3 items-center">
        <MButton variant="tonal" @click="dialogVisible = true">
          {{ t('devUi.dialogDemo') }}
        </MButton>
        <MButton variant="primary" @click="triggerConfirmDialog">
          {{ t('devUi.dialogConfirm') }}
        </MButton>

        <MButton variant="text" @click="toast.info(t('devUi.toastInfo'))">
          {{ t('devUi.toastInfo') }}
        </MButton>
        <MButton variant="text" @click="toast.success(t('devUi.toastSuccess'))">
          {{ t('devUi.toastSuccess') }}
        </MButton>
        <MButton variant="text" @click="toast.danger(t('devUi.toastDanger'))">
          {{ t('devUi.toastDanger') }}
        </MButton>

        <MTooltip text="Tooltip demo text!">
          <span class="text-xs text-textMuted border-b border-dashed border-outline cursor-help">
            Hover for tooltip
          </span>
        </MTooltip>
      </div>
    </div>

    <!-- Empty State -->
    <div class="space-y-3">
      <h3 class="text-base font-semibold text-text">{{ t('devUi.emptyState') }}</h3>
      <MCard>
        <MEmpty
          :title="t('devUi.emptyTitle')"
          :description="t('devUi.emptyDesc')"
        >
          <MButton variant="primary" size="sm">
            {{ t('common.retry') }}
          </MButton>
        </MEmpty>
      </MCard>
    </div>

    <!-- Dialog Demo Component -->
    <MDialog
      v-model:open="dialogVisible"
      :title="t('devUi.dialogDemo')"
      @confirm="dialogVisible = false"
    >
      <p>{{ t('devUi.cardDescription') }}</p>
    </MDialog>

    <!-- Context Menu Component -->
    <MContextMenu
      v-model:open="contextMenuOpen"
      :items="contextMenuItems"
      :x="contextMenuX"
      :y="contextMenuY"
      @select="toast.info(`Selected: ${$event}`)"
    />
  </div>
</template>

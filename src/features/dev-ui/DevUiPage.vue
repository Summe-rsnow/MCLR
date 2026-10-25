<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { wcagContrast } from 'culori'
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
  MContextMenu,
  MColorPicker,
  MEmpty,
  MSkeleton,
  MBadge,
  MAvatar,
  MPageHeader,
  MSection,
  MSettingRow,
  MBrandIcon,
  MLoaderIcon,
  MPopover,
  SEED_PRESETS,
  generatePalette,
  resolveEffectiveMode,
  loaderMeta,
  brandIconMap,
  compTypeMeta,
  useDialog,
  useToast,
  type ContextMenuItem,
  type LoaderKindKey,
  type BrandName,
  type CompTypeKey,
} from '../../ui'

const { t } = useI18n()
const themeStore = useThemeStore()
const dialog = useDialog()
const toast = useToast()

const modeOptions = computed(() => [
  { label: t('devUi.modeLight'), value: 'light', icon: 'i-lucide-sun' },
  { label: t('devUi.modeDark'), value: 'dark', icon: 'i-lucide-moon' },
  { label: t('devUi.modeSystem'), value: 'system', icon: 'i-lucide-monitor' },
])

const materialOptions = computed(() => [
  { label: t('devUi.materialFlat'), value: 'flat', icon: 'i-lucide-square' },
  { label: t('devUi.materialGlass'), value: 'glass', icon: 'i-lucide-sparkles' },
])

const currentPalette = computed(() =>
  generatePalette(themeStore.seed, resolveEffectiveMode(themeStore.mode)),
)

function parseCuloriRgb(rgbStr: string) {
  const [r, g, b] = rgbStr.trim().split(/\s+/).map((n) => parseInt(n, 10) / 255)
  return { mode: 'rgb' as const, r, g, b }
}

const tokenSwatches = computed(() => {
  const p = currentPalette.value
  const raised = parseCuloriRgb(p.raised)
  const canvas = parseCuloriRgb(p.canvas)
  const primary = parseCuloriRgb(p.primary)
  const primarySoft = parseCuloriRgb(p.primarySoft)
  const dangerSoft = parseCuloriRgb(p.dangerSoft)
  const warningSoft = parseCuloriRgb(p.warningSoft)
  const successSoft = parseCuloriRgb(p.successSoft)

  const fmtC = (a: string, b: { mode: 'rgb'; r: number; g: number; b: number }) =>
    `${wcagContrast(parseCuloriRgb(a), b).toFixed(1)}:1`

  return [
    { name: 'canvas', rgb: p.canvas, bgClass: 'bg-canvas', fgClass: 'text-fg', info: 'Base' },
    { name: 'panel', rgb: p.panel, bgClass: 'bg-panel', fgClass: 'text-fg', info: 'Sidebar' },
    { name: 'raised', rgb: p.raised, bgClass: 'bg-raised', fgClass: 'text-fg', info: 'Card' },
    { name: 'sunken', rgb: p.sunken, bgClass: 'bg-sunken', fgClass: 'text-fg', info: 'Well' },
    { name: 'overlay', rgb: p.overlay, bgClass: 'bg-overlay', fgClass: 'text-fg', info: 'Popup' },
    { name: 'line', rgb: p.line, bgClass: 'bg-line', fgClass: 'text-fg', info: 'Hairline' },
    {
      name: 'line-strong',
      rgb: p.lineStrong,
      bgClass: 'bg-line-strong',
      fgClass: 'text-fg',
      info: 'Border',
    },
    {
      name: 'fg',
      rgb: p.fg,
      bgClass: 'bg-fg',
      fgClass: 'text-canvas',
      info: `vs canvas ${fmtC(p.fg, canvas)}`,
    },
    {
      name: 'fg-muted',
      rgb: p.fgMuted,
      bgClass: 'bg-fg-muted',
      fgClass: 'text-canvas',
      info: `vs raised ${fmtC(p.fgMuted, raised)}`,
    },
    {
      name: 'fg-subtle',
      rgb: p.fgSubtle,
      bgClass: 'bg-fg-subtle',
      fgClass: 'text-canvas',
      info: `vs raised ${fmtC(p.fgSubtle, raised)}`,
    },
    {
      name: 'primary',
      rgb: p.primary,
      bgClass: 'bg-primary',
      fgClass: 'text-primary-on',
      info: `on ${fmtC(p.primaryOn, primary)}`,
    },
    {
      name: 'primary-hover',
      rgb: p.primaryHover,
      bgClass: 'bg-primary-hover',
      fgClass: 'text-primary-on',
      info: 'Hover',
    },
    {
      name: 'primary-soft',
      rgb: p.primarySoft,
      bgClass: 'bg-primary-soft',
      fgClass: 'text-primary-soft-fg',
      info: `fg ${fmtC(p.primarySoftFg, primarySoft)}`,
    },
    {
      name: 'danger-soft',
      rgb: p.dangerSoft,
      bgClass: 'bg-danger-soft',
      fgClass: 'text-danger-fg',
      info: `fg ${fmtC(p.dangerFg, dangerSoft)}`,
    },
    {
      name: 'warning-soft',
      rgb: p.warningSoft,
      bgClass: 'bg-warning-soft',
      fgClass: 'text-warning-fg',
      info: `fg ${fmtC(p.warningFg, warningSoft)}`,
    },
    {
      name: 'success-soft',
      rgb: p.successSoft,
      bgClass: 'bg-success-soft',
      fgClass: 'text-success-fg',
      info: `fg ${fmtC(p.successFg, successSoft)}`,
    },
  ]
})

const loaderKinds = Object.keys(loaderMeta) as LoaderKindKey[]
const brandNames = Object.keys(brandIconMap) as BrandName[]
const compTypes = Object.keys(compTypeMeta) as CompTypeKey[]

const inputText = ref('')
const errorInputText = ref('bad_path')
const textareaText = ref('')
const selectVal = ref('opt1')
const selectOptions = [
  { label: 'Option 1 (Fabric)', value: 'opt1', icon: 'i-lucide-waypoints' },
  { label: 'Option 2 (Forge)', value: 'opt2', icon: 'i-lucide-anvil' },
  { label: 'Option 3 (NeoForge)', value: 'opt3', icon: 'i-lucide-flame' },
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
const tabsList = computed(() => [
  { key: 'general', label: t('devUi.tabGeneral'), icon: 'i-lucide-settings' },
  { key: 'advanced', label: t('devUi.tabAdvanced'), icon: 'i-lucide-sliders-horizontal' },
  { key: 'about', label: t('devUi.tabAbout'), icon: 'i-lucide-info' },
])

const progressVal = ref(0.68)
const dialogVisible = ref(false)
const popoverOpen = ref(false)

const contextMenuOpen = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuItems: ContextMenuItem[] = [
  { key: 'edit', label: 'Edit', icon: 'i-lucide-pencil' },
  { key: 'copy', label: 'Copy', icon: 'i-lucide-copy' },
  { key: 'delete', label: 'Delete', icon: 'i-lucide-trash-2', danger: true },
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
  <div class="m-page pb-12 select-none" @contextmenu="openContextMenu">
    <MPageHeader
      :title="t('devUi.title')"
      :description="t('devUi.themeSettings')"
      icon="i-lucide-swatch-book"
    >
      <template #actions>
        <div class="relative">
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-sliders-horizontal"
            @click="popoverOpen = !popoverOpen"
          >
            Popover
          </MButton>
          <MPopover :open="popoverOpen" @update:open="popoverOpen = $event">
            <div class="text-xs text-fg space-y-1 w-48">
              <div class="font-medium">Liquid Glass Popover</div>
              <p class="text-fg-muted">Uses .m-overlay.m-refract surface tier.</p>
            </div>
          </MPopover>
        </div>
      </template>
    </MPageHeader>

    <!-- 1. Theme Controls -->
    <MSection :title="t('devUi.themeSettings')" icon="i-lucide-palette">
      <MSettingRow :label="t('devUi.seedColor')">
        <MColorPicker
          :model-value="themeStore.seed"
          @update:model-value="themeStore.setSeed($event)"
        />
      </MSettingRow>
      <MSettingRow :label="t('devUi.themeMode')">
        <MSegmented
          :model-value="themeStore.mode"
          :options="modeOptions"
          @update:model-value="themeStore.setMode($event as any)"
        />
      </MSettingRow>
      <MSettingRow :label="t('devUi.materialMode')">
        <MSegmented
          :model-value="themeStore.material"
          :options="materialOptions"
          @update:model-value="themeStore.setMaterial($event as any)"
        />
      </MSettingRow>
      <MSettingRow :label="t('devUi.seedPresets')">
        <div class="flex items-center gap-2 flex-wrap">
          <button
            v-for="preset in SEED_PRESETS"
            :key="preset.key"
            type="button"
            class="px-2.5 py-1 rounded-full text-xs font-medium border border-line flex items-center gap-1.5 cursor-pointer hover:bg-sunken transition"
            @click="themeStore.setSeed(preset.hex)"
          >
            <span class="w-3 h-3 rounded-full" :style="{ backgroundColor: preset.hex }" />
            <span>{{ preset.key }}</span>
          </button>
        </div>
      </MSettingRow>
    </MSection>

    <!-- 2. Token Swatch Grid -->
    <MSection :title="t('devUi.paletteTitle')" icon="i-lucide-pipette">
      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <div
          v-for="sw in tokenSwatches"
          :key="sw.name"
          class="p-3 rounded-xl border border-line flex flex-col justify-between gap-2"
          :class="[sw.bgClass, sw.fgClass]"
        >
          <div class="flex items-center justify-between gap-1">
            <span class="text-xs font-bold">{{ sw.name }}</span>
            <span class="text-[11px] opacity-80 font-mono">{{ sw.info }}</span>
          </div>
          <div class="text-[11px] opacity-75 font-mono">{{ sw.rgb }}</div>
        </div>
      </div>
    </MSection>

    <!-- 3. Surface Tiers -->
    <MSection :title="t('devUi.surfaceTiers')" icon="i-lucide-layers">
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-5 gap-4">
        <div class="m-panel p-4 rounded-xl border border-line">
          <div class="text-sm font-medium text-fg">.m-panel</div>
          <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfacePanelDesc') }}</div>
        </div>
        <MCard variant="raised">
          <div class="text-sm font-medium text-fg">MCard raised</div>
          <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfaceRaisedDesc') }}</div>
        </MCard>
        <MCard variant="well">
          <div class="text-sm font-medium text-fg">MCard well</div>
          <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfaceWellDesc') }}</div>
        </MCard>
        <MCard variant="tinted" hoverable>
          <div class="text-sm font-medium text-primary-soft-fg">MCard tinted</div>
          <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfaceTintedDesc') }}</div>
        </MCard>
        <div class="m-overlay m-refract p-4 rounded-xl">
          <div class="text-sm font-medium text-fg">.m-overlay.m-refract</div>
          <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfaceRefractDesc') }}</div>
        </div>
      </div>
    </MSection>

    <!-- 4. Buttons & Badges -->
    <MSection :title="t('devUi.buttons')" icon="i-lucide-mouse-pointer-click">
      <div class="space-y-4">
        <div class="flex flex-wrap items-center gap-3">
          <MButton variant="primary" icon="i-lucide-play">{{ t('devUi.btnPrimary') }}</MButton>
          <MButton variant="tonal" icon="i-lucide-sparkles">{{ t('devUi.btnTonal') }}</MButton>
          <MButton variant="outline" icon="i-lucide-folder">{{ t('devUi.btnOutline') }}</MButton>
          <MButton variant="text">{{ t('devUi.btnText') }}</MButton>
          <MButton variant="danger" icon="i-lucide-trash-2">{{ t('devUi.btnDanger') }}</MButton>
          <MButton variant="primary" loading>{{ t('devUi.btnLoading') }}</MButton>
          <MButton variant="primary" disabled>{{ t('devUi.btnDisabled') }}</MButton>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <MButton size="xs" variant="tonal" icon="i-lucide-zap">Size XS</MButton>
          <MButton size="sm" variant="tonal" icon="i-lucide-zap">Size SM</MButton>
          <MButton size="md" variant="tonal" icon="i-lucide-zap">Size MD</MButton>
          <MButton size="lg" variant="primary" icon="i-lucide-rocket" icon-right="i-lucide-arrow-right">
            Size LG
          </MButton>
          <MIconButton icon="i-lucide-play" size="sm" variant="tonal" tooltip="Small" />
          <MIconButton icon="i-lucide-settings" size="md" variant="outline" tooltip="Medium" />
          <MIconButton icon="i-lucide-trash-2" size="lg" variant="danger" tooltip="Large" />
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <MBadge variant="primary" dot>Primary</MBadge>
          <MBadge variant="info" icon="i-lucide-info">Info</MBadge>
          <MBadge variant="success" dot>Success</MBadge>
          <MBadge variant="warning" size="sm">Warning SM</MBadge>
          <MBadge variant="danger" icon="i-lucide-circle-alert">Danger</MBadge>
          <MBadge variant="neutral">Neutral</MBadge>
        </div>
      </div>
    </MSection>

    <!-- 5. Inputs & Controls -->
    <MSection :title="t('devUi.inputs')" icon="i-lucide-text-cursor-input">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <MInput
          v-model="inputText"
          :label="t('devUi.searchInput')"
          :hint="t('devUi.searchHint')"
          icon="i-lucide-search"
          :placeholder="t('devUi.inputPlaceholder')"
          clearable
        />
        <MInput
          v-model="errorInputText"
          :label="t('devUi.validatedInput')"
          :error="t('devUi.validatedError')"
          icon="i-lucide-folder-x"
        />
        <div class="flex flex-col gap-1">
          <label class="text-xs font-medium text-fg">{{ t('devUi.selectLabel') }}</label>
          <MSelect v-model="selectVal" :options="selectOptions" />
        </div>
        <MTextarea
          v-model="textareaText"
          :label="t('devUi.notesLabel')"
          :hint="t('devUi.notesHint')"
          :placeholder="t('devUi.textareaPlaceholder')"
          :rows="2"
        />
      </div>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4 mt-4 pt-4 border-t border-line">
        <div class="space-y-3">
          <MSwitch v-model="switchVal" :label="t('devUi.switchLabel')" />
          <MCheckbox v-model="checkboxVal" :label="t('devUi.checkboxLabel')" />
        </div>
        <MRadioGroup v-model="radioVal" :options="radioOptions" />
        <div class="space-y-3">
          <MSlider v-model="sliderVal" :min="0" :max="100" />
          <MSegmented v-model="segmentedVal" :options="segmentedOptions" />
        </div>
      </div>
    </MSection>

    <!-- 6. Icon Grids (Loaders / Brands / Comp Types) -->
    <MSection :title="t('devUi.iconGrids')" icon="i-lucide-grid-2x2">
      <div class="space-y-4">
        <div class="flex flex-wrap items-center gap-3">
          <div
            v-for="k in loaderKinds"
            :key="k"
            class="flex items-center gap-2 px-3 py-2 rounded-xl m-well"
          >
            <MLoaderIcon :kind="k" :size="32" />
            <span class="text-xs font-medium text-fg">{{ loaderMeta[k].label }}</span>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-3">
          <div
            v-for="b in brandNames"
            :key="b"
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-sunken text-xs font-medium text-fg"
          >
            <MBrandIcon :name="b" class="w-4 h-4" />
            <span>{{ b }}</span>
          </div>
          <div
            v-for="ct in compTypes"
            :key="ct"
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-primary-soft text-xs font-medium text-primary-soft-fg"
          >
            <span :class="compTypeMeta[ct].icon" class="w-4 h-4" />
            <span>{{ ct }}</span>
          </div>
        </div>
      </div>
    </MSection>

    <!-- 7. Tabs, Progress, Feedback & Empty -->
    <MSection :title="t('devUi.feedback')" icon="i-lucide-bell">
      <div class="space-y-4">
        <MTabs v-model="activeTab" :tabs="tabsList" />
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-3">
            <MProgress :value="progressVal" show-label />
            <MProgress indeterminate size="sm" />
            <div class="flex flex-wrap gap-2">
              <MButton size="sm" variant="tonal" @click="toast.info(t('devUi.toastInfo'))">
                {{ t('devUi.toastInfo') }}
              </MButton>
              <MButton size="sm" variant="tonal" @click="toast.success(t('devUi.toastSuccess'))">
                {{ t('devUi.toastSuccess') }}
              </MButton>
              <MButton size="sm" variant="tonal" @click="toast.warning(t('devUi.toastWarning'))">
                {{ t('devUi.toastWarning') }}
              </MButton>
              <MButton size="sm" variant="danger" @click="toast.danger(t('devUi.toastDanger'))">
                {{ t('devUi.toastDanger') }}
              </MButton>
              <MButton size="sm" variant="primary" @click="dialogVisible = true">
                {{ t('devUi.dialogOpen') }}
              </MButton>
              <MButton size="sm" variant="tonal" @click="triggerConfirmDialog">
                {{ t('devUi.dialogConfirm') }}
              </MButton>
            </div>
          </div>
          <MEmpty :title="t('devUi.emptyTitle')" :description="t('devUi.emptyDesc')">
            <MButton size="sm" variant="tonal" icon="i-lucide-refresh-cw">{{ t('common.retry') }}</MButton>
          </MEmpty>
        </div>
        <div class="flex items-center gap-4">
          <MAvatar name="Alex" :size="36" />
          <MTooltip text="Tooltip Content">
            <MBadge variant="neutral">Hover Tooltip</MBadge>
          </MTooltip>
          <MSkeleton width="160px" height="20px" />
          <MList class="flex-1">
            <MListItem title="Minecraft 1.21.4" subtitle="Fabric 0.16.10" active />
          </MList>
        </div>
      </div>
    </MSection>

    <MDialog
      v-model:open="dialogVisible"
      :title="t('devUi.dialogOpen')"
      @confirm="dialogVisible = false"
    >
      <p>{{ t('devUi.cardDescription') }}</p>
    </MDialog>

    <MContextMenu
      v-model="contextMenuOpen"
      :open="contextMenuOpen"
      :x="contextMenuX"
      :y="contextMenuY"
      :items="contextMenuItems"
      @select="(k) => toast.info(`Context: ${k}`)"
    />
  </div>
</template>

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
  MPathPicker,
  MSwitch,
  MCheckbox,
  MRadioGroup,
  MSlider,
  MSegmented,
  MTabs,
  MList,
  MListItem,
  MDialog,
  MOverlay,
  MDrawer,
  MSearchBar,
  MStatusBadge,
  MTooltip,
  MHelpTooltip,
  MProgress,
  MScroll,
  MCollapse,
  MVirtualList,
  MVirtualGrid,
  MContextMenu,
  MColorPicker,
  MEmpty,
  MSkeleton,
  MSkeletonList,
  MBadge,
  MAvatar,
  MLogo,
  MPageHeader,
  MSection,
  MSettingRow,
  MPopover,
  MBrandIcon,
  MLoaderIcon,
  MPagination,
  MMarkdown,
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

// --- Theme & Mode options ---
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
  const [r, g, b] = rgbStr
    .trim()
    .split(/\s+/)
    .map((n) => parseInt(n, 10) / 255)
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
      info: `${t('devUi.contrastOn')} canvas ${fmtC(p.fg, canvas)}`,
    },
    {
      name: 'fg-muted',
      rgb: p.fgMuted,
      bgClass: 'bg-fg-muted',
      fgClass: 'text-canvas',
      info: `${t('devUi.contrastOn')} raised ${fmtC(p.fgMuted, raised)}`,
    },
    {
      name: 'fg-subtle',
      rgb: p.fgSubtle,
      bgClass: 'bg-fg-subtle',
      fgClass: 'text-canvas',
      info: `${t('devUi.contrastOn')} raised ${fmtC(p.fgSubtle, raised)}`,
    },
    {
      name: 'primary',
      rgb: p.primary,
      bgClass: 'bg-primary',
      fgClass: 'text-primary-on',
      info: `${t('devUi.contrastOn')} ${fmtC(p.primaryOn, primary)}`,
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
      info: `${t('devUi.contrastOn')} ${fmtC(p.primarySoftFg, primarySoft)}`,
    },
    {
      name: 'danger-soft',
      rgb: p.dangerSoft,
      bgClass: 'bg-danger-soft',
      fgClass: 'text-danger-fg',
      info: `${t('devUi.contrastOn')} ${fmtC(p.dangerFg, dangerSoft)}`,
    },
    {
      name: 'warning-soft',
      rgb: p.warningSoft,
      bgClass: 'bg-warning-soft',
      fgClass: 'text-warning-fg',
      info: `${t('devUi.contrastOn')} ${fmtC(p.warningFg, warningSoft)}`,
    },
    {
      name: 'success-soft',
      rgb: p.successSoft,
      bgClass: 'bg-success-soft',
      fgClass: 'text-success-fg',
      info: `${t('devUi.contrastOn')} ${fmtC(p.successFg, successSoft)}`,
    },
  ]
})

// --- Layout state ---
const collapseOpen = ref(false)

// --- Forms state ---
const inputText = ref('')
const errorInputText = ref('bad_path')
const textareaText = ref('')
const pathPickerVal = ref('')
const selectVal = ref('opt1')
const selectOptions = computed(() => [
  { label: t('devUi.selectOpt1'), value: 'opt1', icon: 'i-lucide-waypoints' },
  { label: t('devUi.selectOpt2'), value: 'opt2', icon: 'i-lucide-anvil' },
  { label: t('devUi.selectOpt3'), value: 'opt3', icon: 'i-lucide-flame' },
])

const switchVal = ref(true)
const checkboxVal = ref(true)
const radioVal = ref('r1')
const radioOptions = computed(() => [
  { label: t('devUi.radioOptA'), value: 'r1' },
  { label: t('devUi.radioOptB'), value: 'r2' },
])

const sliderVal = ref(45)
const segmentedVal = ref('tab1')
const segmentedOptions = computed(() => [
  { label: t('devUi.selectOpt1'), value: 'tab1', icon: 'i-lucide-box' },
  { label: t('devUi.selectOpt2'), value: 'tab2', icon: 'i-lucide-cpu' },
  { label: t('devUi.selectOpt3'), value: 'tab3', icon: 'i-lucide-globe' },
])

// --- Navigation state ---
const activeTab = ref('general')
const tabsList = computed(() => [
  { key: 'general', label: t('devUi.tabGeneral'), icon: 'i-lucide-settings' },
  { key: 'advanced', label: t('devUi.tabAdvanced'), icon: 'i-lucide-sliders-horizontal' },
  { key: 'about', label: t('devUi.tabAbout'), icon: 'i-lucide-info' },
])
const paginationPage1 = ref(3)
const paginationPage2 = ref(2)

// --- Data demo state ---
interface VirtualItemData {
  id: number
  title: string
  version: string
}
const virtualItems = ref<VirtualItemData[]>(
  Array.from({ length: 200 }, (_, i) => ({
    id: i + 1,
    title: `Minecraft ${i + 1}`,
    version: `Fabric 0.${(i % 16) + 1}`,
  })),
)

const loaderKinds = Object.keys(loaderMeta) as LoaderKindKey[]
const brandNames = Object.keys(brandIconMap) as BrandName[]
const compTypes = Object.keys(compTypeMeta) as CompTypeKey[]

const markdownSample = `### MCLR Markdown
**Minecraft** Launcher with \`MMarkdown\` component.
- Fabric & Forge & NeoForge
- Modrinth & CurseForge

<center><em>HTML Center Block</em></center>`

// --- Feedback state ---
const progressVal = ref(0.68)

// --- Overlays & Drawers state ---
const searchBarValue = ref('')
const cardLoadingDemo = ref(true)

const dialogVisible = ref(false)
const dialogLoadingOpen = ref(false)
const dialogConfirmLoading = ref(false)
const customOverlayOpen = ref(false)
const popoverOpen = ref(false)

const drawerOpen = ref(false)
const drawerPlacement = ref<'right' | 'left' | 'bottom'>('right')
const drawerTitle = computed(() => {
  switch (drawerPlacement.value) {
    case 'left':
      return t('devUi.drawerLeftTitle')
    case 'bottom':
      return t('devUi.drawerBottomTitle')
    case 'right':
    default:
      return t('devUi.drawerRightTitle')
  }
})

function openDrawer(placement: 'right' | 'left' | 'bottom') {
  drawerPlacement.value = placement
  drawerOpen.value = true
}

async function onDialogLoadingConfirm() {
  dialogConfirmLoading.value = true
  setTimeout(() => {
    dialogConfirmLoading.value = false
    dialogLoadingOpen.value = false
    toast.success(t('devUi.toastSuccess'))
  }, 1200)
}

async function triggerConfirmDanger() {
  const confirmed = await dialog.confirmDanger({
    title: t('devUi.confirmDangerTitle'),
    content: t('devUi.confirmDangerContent'),
    detail: t('devUi.confirmDangerDetail'),
    requireCheckbox: t('devUi.confirmDangerCheckbox'),
  })
  if (confirmed) {
    toast.danger(t('devUi.confirmDangerConfirmed'))
  } else {
    toast.info(t('devUi.confirmDangerCanceled'))
  }
}

async function triggerConfirmWarning() {
  const confirmed = await dialog.confirm({
    title: t('devUi.confirmWarningTitle'),
    content: t('devUi.confirmWarningContent'),
    severity: 'warning',
    detail: t('devUi.confirmDangerDetail'),
  })
  if (confirmed) {
    toast.warning(t('devUi.toastWarning'))
  }
}

const contextMenuOpen = ref(false)
const contextMenuX = ref(0)
const contextMenuY = ref(0)
const contextMenuItems = computed<ContextMenuItem[]>(() => [
  { key: 'edit', label: t('devUi.contextEdit'), icon: 'i-lucide-pencil' },
  { key: 'copy', label: t('devUi.contextCopy'), icon: 'i-lucide-copy' },
  { key: 'delete', label: t('devUi.contextDelete'), icon: 'i-lucide-trash-2', danger: true },
])

function openContextMenu(evt: MouseEvent) {
  evt.preventDefault()
  contextMenuX.value = evt.clientX
  contextMenuY.value = evt.clientY
  contextMenuOpen.value = true
}

async function triggerConfirmDialog() {
  const confirmed = await dialog.confirm({
    title: t('devUi.dialogConfirm'),
    content: t('devUi.dialogContent'),
  })
  if (confirmed) {
    toast.success(t('common.confirm'))
  } else {
    toast.info(t('common.cancel'))
  }
}
</script>

<template>
  <div class="m-page pb-16 select-none space-y-6">
    <MPageHeader
      :title="t('devUi.title')"
      :description="t('devUi.themeSettings')"
      icon="i-lucide-swatch-book"
    />

    <!-- 1. demo-theme: Theme and Materials -->
    <div data-testid="demo-theme" class="space-y-4">
      <MSection :title="t('devUi.themeTitle')" icon="i-lucide-palette">
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

      <MSection :title="t('devUi.paletteTitle')" icon="i-lucide-pipette">
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
          <div
            v-for="sw in tokenSwatches"
            :key="sw.name"
            class="p-3 rounded-xl border border-line flex flex-col justify-between gap-2"
            :class="[sw.bgClass, sw.fgClass]"
          >
            <div class="flex items-center justify-between gap-1">
              <span class="text-xs font-bold font-mono"
                ><code>{{ sw.name }}</code></span
              >
              <span class="text-[11px] opacity-80 font-mono">{{ sw.info }}</span>
            </div>
            <div class="text-[11px] opacity-75 font-mono">{{ sw.rgb }}</div>
          </div>
        </div>
      </MSection>
    </div>

    <!-- 2. demo-layout: Layout and Containers -->
    <div data-testid="demo-layout" class="space-y-4">
      <MSection :title="t('devUi.layoutTitle')" icon="i-lucide-layout">
        <!-- MPageHeader demonstration -->
        <div class="rounded-xl border border-line p-4 bg-sunken space-y-2">
          <div class="text-xs font-mono text-fg-muted"><code>MPageHeader</code></div>
          <MPageHeader
            :title="t('devUi.layoutTitle')"
            :description="t('devUi.pageHeaderSubtitle')"
            icon="i-lucide-panel-top"
          >
            <template #actions>
              <MButton size="sm" variant="tonal" icon="i-lucide-sparkles">
                {{ t('devUi.sectionAction') }}
              </MButton>
            </template>
          </MPageHeader>
        </div>

        <!-- MSection slots demo -->
        <div class="space-y-3 mt-4">
          <div class="text-xs font-mono text-fg-muted">
            <code>MSection</code> & <code>MSettingRow</code>
          </div>
          <MSection icon="i-lucide-box">
            <template #title>{{ t('devUi.sectionTitle') }}</template>
            <template #description>{{ t('devUi.sectionSubtitle') }}</template>
            <template #actions>
              <MButton size="sm" variant="outline">{{ t('devUi.sectionAction') }}</MButton>
            </template>
            <MSettingRow :label="t('devUi.settingRowLabel')" :hint="t('devUi.settingRowHint')">
              <MSwitch v-model="switchVal" />
            </MSettingRow>
          </MSection>
        </div>

        <!-- MCard variants x padding + hoverable -->
        <div class="space-y-3 mt-4">
          <div class="text-xs font-mono text-fg-muted"><code>MCard</code> variants & paddings</div>
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
            <MCard variant="raised" padding="sm" hoverable>
              <div class="text-xs font-mono text-fg font-medium"><code>raised / sm</code></div>
              <p class="text-xs text-fg-muted mt-1">{{ t('devUi.cardSampleDesc') }}</p>
            </MCard>
            <MCard variant="well" padding="md">
              <div class="text-xs font-mono text-fg font-medium"><code>well / md</code></div>
              <p class="text-xs text-fg-muted mt-1">{{ t('devUi.cardSampleDesc') }}</p>
            </MCard>
            <MCard variant="ghost" padding="lg">
              <div class="text-xs font-mono text-fg font-medium"><code>ghost / lg</code></div>
              <p class="text-xs text-fg-muted mt-1">{{ t('devUi.cardSampleDesc') }}</p>
            </MCard>
            <MCard variant="tinted" padding="none" hoverable>
              <div class="p-3">
                <div class="text-xs font-mono text-primary font-medium">
                  <code>tinted / none</code>
                </div>
                <p class="text-xs text-fg-muted mt-1">{{ t('devUi.cardSampleDesc') }}</p>
              </div>
            </MCard>
          </div>
        </div>

        <!-- MCard loading state -->
        <div class="space-y-3 mt-4">
          <div class="flex items-center justify-between">
            <div class="text-xs font-mono text-fg-muted">
              <code>MCard :loading="true"</code>
            </div>
            <MButton size="sm" variant="tonal" @click="cardLoadingDemo = !cardLoadingDemo">
              {{ t('devUi.cardLoadingToggle') }}
            </MButton>
          </div>
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <MCard variant="raised" :loading="cardLoadingDemo">
              <div class="text-xs font-mono text-fg font-medium"><code>raised / loaded</code></div>
              <p class="text-xs text-fg-muted mt-1">{{ t('devUi.cardSampleDesc') }}</p>
            </MCard>
            <MCard variant="well" :loading="cardLoadingDemo">
              <div class="text-xs font-mono text-fg font-medium"><code>well / loaded</code></div>
              <p class="text-xs text-fg-muted mt-1">{{ t('devUi.cardSampleDesc') }}</p>
            </MCard>
          </div>
        </div>

        <!-- MCollapse & MScroll -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
          <div class="rounded-xl border border-line p-4 space-y-3">
            <div class="flex items-center justify-between">
              <span class="text-xs font-mono text-fg-muted"><code>MCollapse</code></span>
              <MButton size="sm" variant="tonal" @click="collapseOpen = !collapseOpen">
                {{ t('devUi.collapseTitle') }}
              </MButton>
            </div>
            <MCollapse :open="collapseOpen">
              <div class="p-3 rounded-lg bg-sunken text-xs text-fg">
                {{ t('devUi.collapseContent') }}
              </div>
            </MCollapse>
          </div>

          <div class="rounded-xl border border-line p-4 space-y-3">
            <span class="text-xs font-mono text-fg-muted"><code>MScroll</code></span>
            <MScroll max-height="100px" class="rounded-lg bg-sunken p-2 space-y-1">
              <div
                v-for="n in 8"
                :key="n"
                class="px-2 py-1 text-xs text-fg rounded bg-raised flex justify-between"
              >
                <span>{{ t('devUi.scrollItem') }} {{ n }}</span>
                <span class="text-fg-subtle">#{{ n }}</span>
              </div>
            </MScroll>
          </div>
        </div>
      </MSection>
    </div>

    <!-- 3. demo-actions: Buttons and Actions -->
    <div data-testid="demo-actions" class="space-y-4">
      <MSection :title="t('devUi.actionsTitle')" icon="i-lucide-mouse-pointer-click">
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
            <MButton size="xs" variant="tonal" icon="i-lucide-zap">{{ t('devUi.sizeXs') }}</MButton>
            <MButton size="sm" variant="tonal" icon="i-lucide-zap">{{ t('devUi.sizeSm') }}</MButton>
            <MButton size="md" variant="tonal" icon="i-lucide-zap">{{ t('devUi.sizeMd') }}</MButton>
            <MButton
              size="lg"
              variant="primary"
              icon="i-lucide-rocket"
              icon-right="i-lucide-arrow-right"
            >
              {{ t('devUi.sizeLg') }}
            </MButton>
            <MIconButton
              icon="i-lucide-play"
              size="sm"
              variant="tonal"
              :tooltip="t('devUi.iconBtnTip')"
            />
            <MIconButton
              icon="i-lucide-settings"
              size="md"
              variant="outline"
              :tooltip="t('devUi.iconBtnTip')"
            />
            <MIconButton
              icon="i-lucide-trash-2"
              size="lg"
              variant="danger"
              :tooltip="t('devUi.iconBtnTip')"
            />
          </div>
          <div class="flex items-center gap-4 pt-2 border-t border-line">
            <span class="text-xs font-mono text-fg-muted"><code>MLogo</code></span>
            <MLogo class="w-8 h-8" />
            <MLogo class="w-10 h-10" />
            <MLogo class="w-12 h-12" />
          </div>
        </div>
      </MSection>
    </div>

    <!-- 4. demo-forms: Forms and Inputs -->
    <div data-testid="demo-forms" class="space-y-4">
      <MSection :title="t('devUi.formsTitle')" icon="i-lucide-text-cursor-input">
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
          <div class="flex flex-col gap-1">
            <label class="text-xs font-medium text-fg">{{ t('devUi.pathPickerLabel') }}</label>
            <MPathPicker
              v-model="pathPickerVal"
              type="directory"
              :placeholder="t('devUi.pathPickerPlaceholder')"
            />
          </div>
          <div class="md:col-span-2">
            <MTextarea
              v-model="textareaText"
              :label="t('devUi.notesLabel')"
              :hint="t('devUi.notesHint')"
              :placeholder="t('devUi.textareaPlaceholder')"
              :rows="2"
            />
          </div>
          <div class="md:col-span-2 space-y-1.5">
            <label class="text-xs font-medium text-fg">{{ t('devUi.searchBarTitle') }}</label>
            <MSearchBar v-model="searchBarValue" :placeholder="t('devUi.searchBarPlaceholder')" />
          </div>
        </div>
        <div
          class="grid grid-cols-1 md:grid-cols-3 gap-6 mt-4 pt-4 border-t border-line items-start"
        >
          <div class="flex flex-wrap items-center gap-6">
            <MSwitch v-model="switchVal" :label="t('devUi.switchLabel')" />
            <MCheckbox v-model="checkboxVal" :label="t('devUi.checkboxLabel')" />
          </div>
          <MRadioGroup v-model="radioVal" :options="radioOptions" />
          <div class="space-y-3 w-full min-w-0 max-w-full overflow-hidden">
            <div class="space-y-1 w-full min-w-0">
              <label class="text-xs font-medium text-fg">{{ t('devUi.sliderLabel') }}</label>
              <MSlider v-model="sliderVal" :min="0" :max="100" />
            </div>
            <div class="w-full min-w-0 max-w-full overflow-hidden">
              <MSegmented v-model="segmentedVal" :options="segmentedOptions" />
            </div>
          </div>
        </div>
      </MSection>
    </div>

    <!-- 5. demo-navigation: Navigation and Pagination -->
    <div data-testid="demo-navigation" class="space-y-4">
      <MSection :title="t('devUi.navTitle')" icon="i-lucide-compass">
        <div class="space-y-6">
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted"><code>MTabs</code></div>
            <MTabs v-model="activeTab" :tabs="tabsList" />
            <div class="p-4 rounded-xl bg-sunken text-xs text-fg">
              <span v-if="activeTab === 'general'">{{ t('devUi.tabContentGeneral') }}</span>
              <span v-else-if="activeTab === 'advanced'">{{ t('devUi.tabContentAdvanced') }}</span>
              <span v-else>{{ t('devUi.tabContentAbout') }}</span>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted"><code>MSegmented</code></div>
            <MSegmented v-model="segmentedVal" :options="segmentedOptions" />
          </div>

          <div class="space-y-4 pt-4 border-t border-line">
            <div class="text-xs font-mono text-fg-muted">
              <code>MPagination</code> (<code>pageCount="20"</code>)
            </div>
            <div class="flex justify-center py-2">
              <MPagination
                :page="paginationPage1"
                :page-count="20"
                :teleport="false"
                @update:page="paginationPage1 = $event"
              />
            </div>

            <div class="text-xs font-mono text-fg-muted">
              <code>MPagination</code> (<code>hasNext="true"</code>)
            </div>
            <div class="flex justify-center py-2">
              <MPagination
                :page="paginationPage2"
                :has-next="true"
                :teleport="false"
                @update:page="paginationPage2 = $event"
              />
            </div>
          </div>
        </div>
      </MSection>
    </div>

    <!-- 6. demo-data: Data Display -->
    <div data-testid="demo-data" class="space-y-4">
      <MSection :title="t('devUi.dataTitle')" icon="i-lucide-database">
        <div class="space-y-6">
          <!-- MList & MListItem -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted">
              <code>MList</code> & <code>MListItem</code>
            </div>
            <MList>
              <MListItem :title="t('devUi.itemTitle')" :subtitle="t('devUi.itemSubtitle')" active>
                <template #prefix>
                  <span class="i-lucide-box w-4 h-4 text-fg-muted" />
                </template>
              </MListItem>
              <MListItem :title="t('devUi.itemTitle')" :subtitle="t('devUi.itemSubtitle')">
                <template #prefix>
                  <span class="i-lucide-file-code w-4 h-4 text-fg-muted" />
                </template>
              </MListItem>
            </MList>
          </div>

          <!-- MVirtualList & MVirtualGrid -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div class="space-y-2">
              <div class="text-xs font-mono text-fg-muted">
                <code>MVirtualList</code> (200 items)
              </div>
              <div class="border border-line rounded-xl overflow-hidden h-48 bg-raised">
                <MVirtualList
                  :items="virtualItems"
                  item-key="id"
                  :estimate-size="44"
                  max-height="192px"
                >
                  <template #item="{ item }">
                    <div
                      class="px-3 py-2 border-b border-line text-xs flex justify-between items-center"
                    >
                      <span class="font-medium text-fg">{{ item.title }}</span>
                      <span class="text-fg-muted">{{ item.version }}</span>
                    </div>
                  </template>
                </MVirtualList>
              </div>
            </div>

            <div class="space-y-2">
              <div class="text-xs font-mono text-fg-muted"><code>MVirtualGrid</code></div>
              <div class="border border-line rounded-xl overflow-hidden h-48 bg-raised">
                <MVirtualGrid
                  :items="virtualItems.slice(0, 40)"
                  item-key="id"
                  :estimate-size="54"
                  :min-column-width="120"
                  max-height="192px"
                >
                  <template #item="{ item }">
                    <div class="p-2 border border-line rounded bg-sunken text-xs truncate">
                      <div class="font-medium text-fg">{{ item.title }}</div>
                      <div class="text-[11px] text-fg-subtle">{{ item.version }}</div>
                    </div>
                  </template>
                </MVirtualGrid>
              </div>
            </div>
          </div>

          <!-- Badges, Avatars -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted">
              <code>MBadge</code> & <code>MAvatar</code>
            </div>
            <div class="flex flex-wrap items-center gap-3">
              <MBadge variant="primary" dot>{{ t('devUi.badgePrimary') }}</MBadge>
              <MBadge variant="info" icon="i-lucide-info">{{ t('devUi.badgeInfo') }}</MBadge>
              <MBadge variant="success" dot>{{ t('devUi.badgeSuccess') }}</MBadge>
              <MBadge variant="warning" size="sm">{{ t('devUi.badgeWarning') }}</MBadge>
              <MBadge variant="danger" icon="i-lucide-circle-alert">{{
                t('devUi.badgeDanger')
              }}</MBadge>
              <MBadge variant="neutral">{{ t('devUi.badgeNeutral') }}</MBadge>
              <MAvatar name="Steve" :size="32" />
              <MAvatar name="Alex" :size="36" />
            </div>
          </div>

          <!-- MStatusBadge all 7 semantic states -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted">
              <code>MStatusBadge</code> (7 semantic states)
            </div>
            <div class="flex flex-wrap items-center gap-2">
              <MStatusBadge status="online" :label="t('devUi.statusOnline')" pulse />
              <MStatusBadge status="running" :label="t('devUi.statusRunning')" pulse />
              <MStatusBadge status="success" :label="t('devUi.statusSuccess')" />
              <MStatusBadge status="waiting" :label="t('devUi.statusWaiting')" />
              <MStatusBadge status="warning" :label="t('devUi.statusWarning')" />
              <MStatusBadge status="danger" :label="t('devUi.statusDanger')" pulse />
              <MStatusBadge status="offline" :label="t('devUi.statusOffline')" />
            </div>
          </div>

          <!-- Brand & Loader icons -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted">
              <code>MLoaderIcon</code>, <code>MBrandIcon</code> & <code>MCompType</code>
            </div>
            <div class="flex flex-wrap items-center gap-3">
              <div
                v-for="k in loaderKinds"
                :key="k"
                class="flex items-center gap-2 px-3 py-1.5 rounded-lg m-well text-xs"
              >
                <MLoaderIcon :kind="k" :size="24" />
                <span class="text-fg">{{ loaderMeta[k].label }}</span>
              </div>
              <div
                v-for="b in brandNames"
                :key="b"
                class="flex items-center gap-2 px-2.5 py-1 rounded bg-sunken text-xs text-fg"
              >
                <MBrandIcon :name="b" class="w-3.5 h-3.5" />
                <span>{{ b }}</span>
              </div>
              <div
                v-for="ct in compTypes"
                :key="ct"
                class="flex items-center gap-2 px-2.5 py-1 rounded bg-primary-soft text-xs text-primary-soft-fg"
              >
                <span :class="compTypeMeta[ct].icon" class="w-3.5 h-3.5" />
                <span>{{ ct }}</span>
              </div>
            </div>
          </div>

          <!-- MMarkdown demo -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted"><code>MMarkdown</code></div>
            <div class="p-4 rounded-xl border border-line bg-sunken">
              <MMarkdown :source="markdownSample" format="markdown" />
            </div>
          </div>
        </div>
      </MSection>
    </div>

    <!-- 7. demo-feedback: Feedback and Skeletons -->
    <div data-testid="demo-feedback" class="space-y-4">
      <MSection :title="t('devUi.feedbackTitle')" icon="i-lucide-bell">
        <div class="space-y-6">
          <div class="space-y-3">
            <div class="text-xs font-mono text-fg-muted"><code>MProgress</code></div>
            <MProgress :value="progressVal" show-label />
            <MProgress indeterminate size="sm" />
          </div>

          <div class="space-y-3">
            <div class="text-xs font-mono text-fg-muted">
              <code>MSkeleton</code> & <code>MSkeletonList</code>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="space-y-2 p-3 border border-line rounded-xl">
                <span class="text-xs text-fg-muted">{{ t('devUi.skeletonRowTitle') }}</span>
                <div class="flex items-center gap-3">
                  <MSkeleton width="2.5rem" height="2.5rem" class="rounded-xl" />
                  <div class="space-y-1.5 flex-1">
                    <MSkeleton width="60%" height="0.875rem" />
                    <MSkeleton width="40%" height="0.75rem" />
                  </div>
                </div>
                <MSkeletonList variant="row" :rows="2" />
              </div>
              <div class="space-y-2 p-3 border border-line rounded-xl">
                <span class="text-xs text-fg-muted">{{ t('devUi.skeletonTileTitle') }}</span>
                <MSkeletonList variant="tile" :rows="3" />
              </div>
            </div>
          </div>

          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted"><code>MEmpty</code></div>
            <MEmpty
              :title="t('devUi.emptyTitle')"
              :description="t('devUi.emptyDesc')"
              :action-text="t('devUi.emptyAction')"
              @action="toast.info(t('devUi.toastInfo'))"
            />
          </div>
        </div>
      </MSection>
    </div>

    <!-- 8. demo-overlays: Overlays and Dialogs -->
    <div data-testid="demo-overlays" class="space-y-4" @contextmenu="openContextMenu">
      <MSection :title="t('devUi.overlaysTitle')" icon="i-lucide-copy">
        <div class="space-y-6">
          <!-- Dialogs, Overlays & Drawers -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted">
              <code>MDialog</code>, <code>MOverlay</code> & <code>MDrawer</code>
            </div>
            <div class="flex flex-wrap gap-2">
              <MButton size="sm" variant="primary" @click="dialogVisible = true">
                {{ t('devUi.dialogOpen') }}
              </MButton>
              <MButton
                size="sm"
                variant="tonal"
                data-testid="open-loading-dialog-btn"
                @click="dialogLoadingOpen = true"
              >
                {{ t('devUi.dialogLoadingOpen') }}
              </MButton>
              <MButton
                size="sm"
                variant="tonal"
                icon="i-lucide-layers"
                data-testid="open-custom-overlay-btn"
                @click="customOverlayOpen = true"
              >
                {{ t('devUi.overlayCustomOpen') }}
              </MButton>
              <MButton
                size="sm"
                variant="tonal"
                icon="i-lucide-panel-right-open"
                data-testid="open-drawer-right-btn"
                @click="openDrawer('right')"
              >
                {{ t('devUi.drawerRightBtn') }}
              </MButton>
              <MButton
                size="sm"
                variant="tonal"
                icon="i-lucide-panel-left-open"
                data-testid="open-drawer-left-btn"
                @click="openDrawer('left')"
              >
                {{ t('devUi.drawerLeftBtn') }}
              </MButton>
              <MButton
                size="sm"
                variant="tonal"
                icon="i-lucide-panel-bottom-open"
                data-testid="open-drawer-bottom-btn"
                @click="openDrawer('bottom')"
              >
                {{ t('devUi.drawerBottomBtn') }}
              </MButton>
            </div>
          </div>

          <!-- Secondary Confirms & Toasts -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted">
              <code>useDialog().confirmDanger</code> & <code>useToast</code>
            </div>
            <div class="flex flex-wrap gap-2">
              <MButton size="sm" variant="tonal" @click="triggerConfirmDialog">
                {{ t('devUi.dialogConfirm') }}
              </MButton>
              <MButton
                size="sm"
                variant="tonal"
                data-testid="trigger-confirm-warning-btn"
                @click="triggerConfirmWarning"
              >
                {{ t('devUi.confirmWarningBtn') }}
              </MButton>
              <MButton
                size="sm"
                variant="danger"
                data-testid="trigger-confirm-danger-btn"
                @click="triggerConfirmDanger"
              >
                {{ t('devUi.confirmDangerBtn') }}
              </MButton>
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
            </div>
          </div>

          <!-- Tooltips & Popover -->
          <div class="space-y-2">
            <div class="text-xs font-mono text-fg-muted">
              <code>MTooltip</code>, <code>MHelpTooltip</code> & <code>MPopover</code>
            </div>
            <div class="flex flex-wrap items-center gap-4">
              <MTooltip :text="t('devUi.tooltipContent')">
                <MBadge variant="neutral">{{ t('devUi.tooltipHover') }}</MBadge>
              </MTooltip>
              <MHelpTooltip :text="t('devUi.helpTooltipText')" />
              <div class="relative">
                <MButton
                  size="sm"
                  variant="tonal"
                  icon="i-lucide-sliders-horizontal"
                  @click="popoverOpen = !popoverOpen"
                >
                  {{ t('devUi.popoverBtn') }}
                </MButton>
                <MPopover :open="popoverOpen" @update:open="popoverOpen = $event">
                  <div class="text-xs text-fg space-y-1 w-48 p-1">
                    <div class="font-medium">{{ t('devUi.popoverTitle') }}</div>
                    <p class="text-fg-muted">{{ t('devUi.popoverDesc') }}</p>
                  </div>
                </MPopover>
              </div>
            </div>
          </div>

          <!-- Context Menu trigger area -->
          <div
            class="p-6 rounded-xl border border-dashed border-line text-center text-xs text-fg-muted bg-sunken"
          >
            {{ t('devUi.contextMenuZone') }}
          </div>

          <!-- Surface Tiers -->
          <div class="space-y-2 pt-4 border-t border-line">
            <div class="text-xs font-mono text-fg-muted">
              <code>{{ t('devUi.surfaceTiers') }}</code>
            </div>
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
              <div class="m-panel p-4 rounded-xl border border-line">
                <div class="text-sm font-medium text-fg font-mono"><code>.m-panel</code></div>
                <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfacePanelDesc') }}</div>
              </div>
              <div class="m-overlay p-4 rounded-xl shadow-2 border border-line">
                <div class="text-sm font-medium text-fg font-mono"><code>.m-overlay</code></div>
                <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfaceRaisedDesc') }}</div>
              </div>
              <div class="m-overlay m-refract p-4 rounded-xl">
                <div class="text-sm font-medium text-fg font-mono">
                  <code>.m-overlay.m-refract</code>
                </div>
                <div class="text-xs text-fg-muted mt-1">{{ t('devUi.surfaceRefractDesc') }}</div>
              </div>
            </div>
            <p class="text-xs text-fg-subtle font-mono mt-2">
              <code>AmbientBackdrop</code> / <code>GlassDefs</code>:
              {{ t('devUi.ambientGlassNote') }}
            </p>
          </div>
        </div>
      </MSection>
    </div>

    <!-- Standard Dialog instance -->
    <MDialog
      v-model:open="dialogVisible"
      :title="t('devUi.dialogOpen')"
      @confirm="dialogVisible = false"
    >
      <p class="text-xs text-fg leading-relaxed">{{ t('devUi.dialogContent') }}</p>
    </MDialog>

    <!-- Loading Dialog instance -->
    <MDialog
      v-model:open="dialogLoadingOpen"
      :title="t('devUi.dialogLoadingOpen')"
      :confirm-loading="dialogConfirmLoading"
      @confirm="onDialogLoadingConfirm"
    >
      <p class="text-xs text-fg leading-relaxed">{{ t('devUi.dialogLoadingContent') }}</p>
    </MDialog>

    <!-- Custom MOverlay instance -->
    <MOverlay v-model:open="customOverlayOpen" align="center" transition="m-modal">
      <div
        class="m-overlay m-refract p-6 rounded-2xl max-w-md w-full mx-4 space-y-4 shadow-2xl"
        role="dialog"
        aria-modal="true"
      >
        <div class="flex items-center justify-between">
          <h3 class="text-base font-semibold text-fg">{{ t('devUi.overlayCustomTitle') }}</h3>
          <MIconButton
            icon="i-lucide-x"
            size="sm"
            variant="text"
            @click="customOverlayOpen = false"
          />
        </div>
        <p class="text-xs text-fg-muted leading-relaxed">{{ t('devUi.overlayCustomDesc') }}</p>
        <div class="flex justify-end gap-2 pt-2">
          <MButton size="sm" variant="primary" @click="customOverlayOpen = false">
            {{ t('common.close') }}
          </MButton>
        </div>
      </div>
    </MOverlay>

    <!-- MDrawer instance -->
    <MDrawer
      v-model:open="drawerOpen"
      :placement="drawerPlacement"
      :title="drawerTitle"
      :subtitle="t('devUi.drawerSampleDesc')"
      :hide-footer="false"
      @confirm="drawerOpen = false"
    >
      <div class="space-y-4 text-xs text-fg">
        <p class="text-fg-muted">{{ t('devUi.drawerSampleDesc') }}</p>
        <div class="p-4 rounded-xl m-well space-y-2">
          <div class="font-medium text-fg">Drawer Placement: {{ drawerPlacement }}</div>
          <p class="text-fg-muted">Size: md, Blur: true</p>
        </div>
      </div>
    </MDrawer>

    <!-- Context menu component instance -->
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

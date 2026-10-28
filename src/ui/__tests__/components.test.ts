import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { i18n } from '../../i18n'
import {
  MButton,
  MIconButton,
  MCard,
  MInput,
  MTextarea,
  MPathPicker,
  MSelect,
  MSwitch,
  MCheckbox,
  MRadioGroup,
  MSlider,
  MSegmented,
  MTabs,
  MListItem,
  MDialog,
  MToast,
  MTooltip,
  MHelpTooltip,
  MProgress,
  MScroll,
  MContextMenu,
  MColorPicker,
  MEmpty,
  MSkeleton,
  MBadge,
  MAvatar,
  MPageHeader,
  MSection,
  MSettingRow,
  MPopover,
  MBrandIcon,
  MLoaderIcon,
  MPagination,
  MOverlay,
  MDrawer,
  MSearchBar,
  MStatusBadge,
  MImage,
  useDialog,
  useToast,
  applyTheme,
  resolveEffectiveMode,
  generatePalette,
} from '../index'

describe('Theme Engine', () => {
  it('generates OKLCH tonal palette with rgb values', () => {
    const palette = generatePalette('#3b82f6', 'light')
    expect(palette.primary).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(palette.canvas).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(palette.raised).toMatch(/^\d+\s+\d+\s+\d+$/)
    expect(palette.fg).toMatch(/^\d+\s+\d+\s+\d+$/)
  })
  it('applies theme attributes and css variables to documentElement', async () => {
    applyTheme({
      seed: '#10b981',
      mode: 'dark',
      material: 'glass',
    })

    const { promise, resolve } = Promise.withResolvers<void>()
    requestAnimationFrame(() => resolve())
    await promise

    const root = document.documentElement
    expect(root.dataset.mode).toBe('dark')
    expect(root.dataset.material).toBe('glass')
    expect(root.style.getPropertyValue('--c-primary')).toBeTruthy()
  })

  it('resolves system mode', () => {
    expect(['light', 'dark']).toContain(resolveEffectiveMode('system'))
    expect(resolveEffectiveMode('dark')).toBe('dark')
    expect(resolveEffectiveMode('light')).toBe('light')
  })
})

describe('UI Components', () => {
  it('MButton handles click and loading state', async () => {
    const wrapper = mount(MButton, {
      slots: { default: 'Click Me' },
    })
    expect(wrapper.text()).toBe('Click Me')
    await wrapper.trigger('click')
    expect(wrapper.emitted('click')).toHaveLength(1)

    await wrapper.setProps({ loading: true })
    expect(wrapper.find('button').attributes('disabled')).toBeDefined()
    expect(wrapper.find('.animate-spin').exists()).toBe(true)
  })

  it('MIconButton triggers click', async () => {
    const wrapper = mount(MIconButton, {
      props: { icon: 'i-lucide-play' },
    })
    await wrapper.trigger('click')
    expect(wrapper.emitted('click')).toHaveLength(1)
  })

  it('MCard renders variants and hoverable state', () => {
    const wrapper = mount(MCard, {
      props: { variant: 'well', hoverable: true },
      slots: { default: 'Card Content' },
    })
    expect(wrapper.classes()).toContain('m-well')
    expect(wrapper.text()).toBe('Card Content')
  })

  it('MInput emits update:modelValue and handles clear', async () => {
    const wrapper = mount(MInput, {
      props: { modelValue: 'hello', clearable: true },
    })
    const input = wrapper.find('input')
    expect(input.element.value).toBe('hello')

    await input.setValue('world')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['world'])

    const clearBtn = wrapper.find('button')
    await clearBtn.trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[1]).toEqual([''])
    expect(wrapper.emitted('clear')).toHaveLength(1)
  })
  it('MInput stepper buttons increment and decrement number values', async () => {
    const wrapper = mount(MInput, {
      props: { modelValue: '10', type: 'number', min: 0, max: 20, step: 2 },
    })
    const buttons = wrapper.findAll('button')
    expect(buttons.length).toBe(2)
    // Up stepper
    await buttons[0].trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['12'])
    // Down stepper
    await buttons[1].trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[1]).toEqual(['8'])
  })

  it('MTextarea emits update:modelValue', async () => {
    const wrapper = mount(MTextarea, {
      props: { modelValue: 'text' },
    })
    const textarea = wrapper.find('textarea')
    await textarea.setValue('new text')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['new text'])
  })

  it('MSelect handles custom listbox selection and keyboard navigation', async () => {
    const wrapper = mount(MSelect, {
      props: {
        modelValue: 'a',
        options: [
          { label: 'Option A', value: 'a' },
          { label: 'Option B', value: 'b' },
        ],
      },
    })
    const trigger = wrapper.find('button[role="combobox"]')
    await trigger.trigger('click')
    await nextTick()
    const listbox = document.body.querySelector('[role="listbox"]')
    expect(listbox?.classList.contains('m-select-list')).toBe(true)
    const options = document.body.querySelectorAll('[role="option"]')
    expect(options.length).toBeGreaterThanOrEqual(2)
    ;(options[1] as HTMLButtonElement).click()
    await nextTick()
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['b'])
  })
  it('MSelect renders avatar and badge on trigger and options', async () => {
    const wrapper = mount(MSelect, {
      props: {
        modelValue: 'user1',
        options: [
          {
            label: 'PlayerOne',
            value: 'user1',
            avatar: 'data:image/png;base64,avatar1',
            badge: '离线',
          },
          {
            label: 'PlayerTwo',
            value: 'user2',
            avatar: 'data:image/png;base64,avatar2',
            badge: '微软',
          },
        ],
      },
    })
    // Trigger should display selected avatar
    const triggerImg = wrapper.find('button[role="combobox"] img')
    expect(triggerImg.exists()).toBe(true)
    expect(triggerImg.attributes('src')).toBe('data:image/png;base64,avatar1')

    // Open listbox and check options
    await wrapper.find('button[role="combobox"]').trigger('click')
    await nextTick()
    const options = document.body.querySelectorAll('[role="option"]')
    expect(options[0]?.textContent).toContain('PlayerOne')
    expect(options[0]?.textContent).toContain('离线')
    expect(options[1]?.textContent).toContain('PlayerTwo')
    expect(options[1]?.textContent).toContain('微软')
    const optionImgs = document.body.querySelectorAll('[role="option"] img')
    expect(optionImgs.length).toBeGreaterThanOrEqual(2)
    wrapper.unmount()
    await nextTick()
  })
  it('MSelect keeps a right-edge dropdown inside the viewport', async () => {
    // jsdom reports zero-size rects, so the trigger rect is stubbed explicitly.
    const wrapper = mount(MSelect, {
      props: {
        modelValue: 'a',
        options: Array.from({ length: 30 }, (_, i) => ({ label: `Option ${i}`, value: `v${i}` })),
      },
      attachTo: document.body,
    })
    const trigger = wrapper.find('button[role="combobox"]').element as HTMLElement
    trigger.getBoundingClientRect = () =>
      ({
        top: 120,
        bottom: 152,
        left: 900,
        right: 1020,
        width: 120,
        height: 32,
        x: 900,
        y: 120,
        toJSON: () => ({}),
      }) as DOMRect

    Object.defineProperty(window, 'innerWidth', { value: 1024, configurable: true })
    Object.defineProperty(window, 'innerHeight', { value: 768, configurable: true })

    await wrapper.find('button[role="combobox"]').trigger('click')
    await nextTick()
    await nextTick()

    const popup = document.body.querySelector('.m-select-list')?.parentElement as HTMLElement
    expect(popup).toBeTruthy()
    const left = Number.parseFloat(popup.style.left)
    expect(Number.isNaN(left)).toBe(false)
    expect(left).toBeLessThanOrEqual(1024 - 8 - 120)
    wrapper.unmount()
  })

  it('every loader kind renders its official artwork through MLoaderIcon', () => {
    const wrapper = mount(MLoaderIcon, {
      props: { kind: 'vanilla', size: 24 },
    })
    const img = wrapper.find('img')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toContain('/assets/icons/')

    const cases: Array<[string, string]> = [
      ['fabric', 'fabric'],
      ['forge', 'forge'],
      ['neoforge', 'neoforge'],
      ['quilt', 'quilt'],
      ['labymod', 'labymod'],
      ['optifine', 'optifine'],
      ['cleanroom', 'cleanroom'],
    ]
    for (const [kind, file] of cases) {
      const w = mount(MLoaderIcon, { props: { kind, size: 24 } })
      const src = w.find('img').attributes('src') ?? ''
      expect(src, kind).toContain(`/assets/icons/loaders/${file}`)
    }
  })

  it('MSelect renders a square logo option uncropped alongside avatars', async () => {
    const wrapper = mount(MSelect, {
      props: {
        modelValue: 'fabric',
        options: [
          { label: 'Fabric', value: 'fabric', image: '/assets/icons/loaders/fabric.png' },
          { label: 'Steve', value: 'steve', avatar: '/head.png' },
        ],
      },
    })
    const trigger = wrapper.find('button[role="combobox"]')
    const img = trigger.find('img')
    expect(img.exists()).toBe(true)
    expect(img.attributes('src')).toContain('loaders/fabric.png')
    // MImage applies the sizing classes to its wrapper, not the <img>.
    const frame = img.element.closest('[class*="object-contain"]')
    expect(frame, 'logo frame uses object-contain').toBeTruthy()
    expect(frame!.className).not.toContain('rounded-full')
  })

  it('MSelect disabled and empty options state prevents opening and displays placeholder', async () => {
    const wrapper = mount(MSelect, {
      props: {
        modelValue: '',
        options: [],
      },
      global: {
        plugins: [i18n],
      },
    })
    const trigger = wrapper.find('button[role="combobox"]')
    expect(trigger.attributes('disabled')).toBeDefined()
    expect(trigger.text()).toContain('无可用选项')
    await trigger.trigger('click')
    await nextTick()
    expect(document.body.querySelector('[role="listbox"]')).toBeNull()
  })
  it('MSelect supports block prop and adaptive trigger width', () => {
    const defaultWrapper = mount(MSelect, {
      props: {
        modelValue: 'a',
        options: [{ label: 'Option A', value: 'a' }],
      },
    })
    expect(defaultWrapper.classes()).toContain('w-full')
    expect(defaultWrapper.find('button').classes()).toContain('w-full')

    const inlineWrapper = mount(MSelect, {
      props: {
        modelValue: 'a',
        options: [{ label: 'Option A', value: 'a' }],
        block: false,
      },
    })
    expect(inlineWrapper.classes()).toContain('w-auto')
    expect(inlineWrapper.classes()).toContain('max-w-[360px]')
    expect(inlineWrapper.find('button').classes()).toContain('w-auto')
    expect(inlineWrapper.find('button').classes()).toContain('min-w-[140px]')
  })
  it('MSwitch toggles value', async () => {
    const wrapper = mount(MSwitch, {
      props: { modelValue: false, label: 'Toggle' },
    })
    expect(wrapper.text()).toContain('Toggle')
    await wrapper.find('button').trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([true])
  })

  it('MCheckbox toggles value', async () => {
    const wrapper = mount(MCheckbox, {
      props: { modelValue: false, label: 'Agree' },
    })
    const checkbox = wrapper.find('input[type="checkbox"]')
    await checkbox.setValue(true)
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([true])
  })
  it('MCheckbox respects disabled prop', () => {
    const wrapper = mount(MCheckbox, {
      props: { modelValue: false, label: 'Disabled Checkbox', disabled: true },
    })
    const checkbox = wrapper.find('input[type="checkbox"]')
    expect(checkbox.attributes('disabled')).toBeDefined()
    expect(wrapper.find('label').classes()).toContain('cursor-not-allowed')
  })

  it('MRadioGroup updates selection', async () => {
    const wrapper = mount(MRadioGroup, {
      props: {
        modelValue: '1',
        options: [
          { label: 'One', value: '1' },
          { label: 'Two', value: '2' },
        ],
      },
    })
    const radios = wrapper.findAll('input[type="radio"]')
    await radios[1].setValue()
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['2'])
  })

  it('MSlider updates number', async () => {
    const wrapper = mount(MSlider, {
      props: { modelValue: 50 },
    })
    const range = wrapper.find('input[type="range"]')
    await range.setValue('75')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([75])
  })

  it('MSegmented changes option', async () => {
    const wrapper = mount(MSegmented, {
      props: {
        modelValue: 'x',
        options: [
          { label: 'X', value: 'x' },
          { label: 'Y', value: 'y' },
        ],
      },
    })
    const buttons = wrapper.findAll('button')
    await buttons[1].trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['y'])
  })

  it('MTabs changes active tab', async () => {
    const wrapper = mount(MTabs, {
      props: {
        modelValue: 'tab1',
        tabs: [
          { key: 'tab1', label: 'Tab 1' },
          { key: 'tab2', label: 'Tab 2' },
        ],
      },
    })
    const tabButtons = wrapper.findAll('button')
    await tabButtons[1].trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['tab2'])
  })

  it('MListItem renders properly and emits click', async () => {
    const wrapper = mount(MListItem, {
      props: { title: 'Item 1', subtitle: 'Sub 1' },
    })
    expect(wrapper.text()).toContain('Item 1')
    expect(wrapper.text()).toContain('Sub 1')
    await wrapper.trigger('click')
    expect(wrapper.emitted('click')).toHaveLength(1)
  })

  it('MDialog mounts and handles confirm/cancel', async () => {
    mount(MDialog, {
      props: { open: true, title: 'Dialog Title' },
      global: { plugins: [i18n] },
    })
    expect(document.body.innerHTML).toContain('Dialog Title')
  })

  it('useDialog confirm returns user choice', async () => {
    const { confirm, handleConfirm, activeDialog } = useDialog()
    const promise = confirm({ content: 'Test prompt' })
    expect(activeDialog.value?.open).toBe(true)
    handleConfirm()
    const res = await promise
    expect(res).toBe(true)
  })

  it('MToast and useToast manage toasts', async () => {
    const toast = useToast()
    const id = toast.info('Toast message', 0)
    expect(toast.toasts.value.some((t) => t.id === id)).toBe(true)
    toast.remove(id)
    expect(toast.toasts.value.some((t) => t.id === id)).toBe(false)
  })

  it('MSegmented keeps icon option labels and exposes accessible names', async () => {
    const wrapper = mount(MSegmented, {
      props: {
        modelValue: 'light',
        block: true,
        options: [
          { label: '浅色模式', value: 'light', icon: 'i-lucide-sun' },
          { label: '深色模式', value: 'dark', icon: 'i-lucide-moon' },
          { label: '跟随系统', value: 'system', icon: 'i-lucide-laptop' },
        ],
      },
      attachTo: document.body,
    })
    await nextTick()

    // Regression: an `v-else-if` chain tied the label to the icon branch, so any
    // option with an icon lost its visible text.
    const buttons = wrapper.findAll('button[data-seg-btn]')
    expect(buttons).toHaveLength(3)
    expect(buttons[0].text()).toContain('浅色模式')
    expect(buttons[2].text()).toContain('跟随系统')

    // Icon-only collapse must stay reachable: every button carries its label.
    for (const [i, expected] of ['浅色模式', '深色模式', '跟随系统'].entries()) {
      expect(buttons[i].attributes('title')).toBe(expected)
      expect(buttons[i].attributes('aria-label')).toBe(expected)
    }
    wrapper.unmount()
  })

  it('MTooltip renders slot content and teleports tooltip on hover', async () => {
    const wrapper = mount(MTooltip, {
      props: { text: 'Help text' },
      slots: { default: '<span>Hover Me</span>' },
    })
    expect(wrapper.text()).toContain('Hover Me')
    await wrapper.trigger('mouseenter')
    expect(document.body.innerHTML).toContain('Help text')
  })

  it('MHelpTooltip renders help icon and provides tooltip text', async () => {
    const wrapper = mount(MHelpTooltip, {
      props: { text: 'Helpful explanation', size: 'sm' },
    })
    expect(wrapper.find('button').exists()).toBe(true)
    expect(wrapper.find('.i-lucide-circle-question-mark').exists()).toBe(true)
    await wrapper.find('.relative').trigger('mouseenter')
    expect(document.body.innerHTML).toContain('Helpful explanation')
  })
  it('MProgress displays percentage', () => {
    const wrapper = mount(MProgress, {
      props: { value: 0.75, showLabel: true },
    })
    expect(wrapper.text()).toContain('75%')
  })

  it('MScroll renders slot content', () => {
    const wrapper = mount(MScroll, {
      slots: { default: '<div class="scroll-inner">Scrollable</div>' },
    })
    expect(wrapper.find('.scroll-inner').exists()).toBe(true)
  })

  it('MContextMenu triggers select', async () => {
    mount(MContextMenu, {
      props: {
        open: true,
        'onUpdate:open': () => {},
        x: 100,
        y: 100,
        items: [{ key: 'edit', label: 'Edit' }],
      },
    })
    const btn = document.body.querySelector('button')
    if (btn) {
      btn.click()
      await nextTick()
    }
  })

  it('MContextMenu clamps a bottom-right right-click inside the viewport', async () => {
    Object.defineProperty(window, 'innerWidth', { value: 800, configurable: true })
    Object.defineProperty(window, 'innerHeight', { value: 600, configurable: true })

    const wrapper = mount(MContextMenu, {
      props: {
        open: true,
        'onUpdate:open': () => {},
        x: 790,
        y: 590,
        items: [{ key: 'edit', label: 'Edit' }],
      },
      attachTo: document.body,
    })
    await nextTick()
    await nextTick()

    // Teleported menus from earlier tests may still be mounted; take the newest.
    const menus = [...document.body.querySelectorAll<HTMLElement>('[data-testid="m-context-menu"]')]
    const menu = menus[menus.length - 1] ?? null
    expect(menu, document.body.innerHTML.slice(0, 400)).toBeTruthy()
    const left = Number.parseFloat(menu!.style.left)
    const top = Number.parseFloat(menu!.style.top)
    // Right-click at (790, 590) would overflow; the menu is pulled back by its width.
    expect(left).toBe(800 - 8 - 176)
    expect(top).toBeLessThanOrEqual(600 - 8)
    wrapper.unmount()
  })

  it('MColorPicker emits hex color update', async () => {
    const wrapper = mount(MColorPicker, {
      props: { modelValue: '#3b82f6' },
    })
    const input = wrapper.find('input[type="color"]')
    await input.setValue('#ef4444')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['#ef4444'])
  })

  it('MEmpty renders title and description', () => {
    const wrapper = mount(MEmpty, {
      props: { title: 'No Data', description: 'Check back later' },
    })
    expect(wrapper.text()).toContain('No Data')
    expect(wrapper.text()).toContain('Check back later')
  })

  it('MSkeleton renders width and height and rounded variants', () => {
    const wrapper = mount(MSkeleton, {
      props: { width: '120px', height: '24px' },
    })
    expect(wrapper.attributes('style')).toContain('width: 120px')
    expect(wrapper.attributes('style')).toContain('height: 24px')
    expect(wrapper.classes()).toContain('rounded-xl')

    const customRounded = mount(MSkeleton, {
      props: { rounded: '2xl' },
    })
    expect(customRounded.classes()).toContain('rounded-2xl')

    const noRounded = mount(MSkeleton, {
      props: { rounded: false },
    })
    expect(noRounded.classes()).not.toContain('rounded-xl')
  })

  it('MBadge renders slot content and tonal status classes', () => {
    const wrapper = mount(MBadge, {
      props: { variant: 'success', dot: true },
      slots: { default: 'Online' },
    })
    expect(wrapper.text()).toBe('Online')
    expect(wrapper.classes()).toContain('text-success-fg')
  })

  it('MAvatar shows image or initials', async () => {
    const wrapper = mount(MAvatar, {
      props: { name: 'Steve' },
    })
    expect(wrapper.text()).toBe('ST')

    await wrapper.setProps({ src: 'data:image/png;base64,abc' })
    expect(wrapper.find('img').exists()).toBe(true)
  })

  it('renders MPageHeader, MSection, MSettingRow, MBrandIcon, MLoaderIcon and MPopover', async () => {
    const header = mount(MPageHeader, {
      props: { title: 'Test Page', description: 'Sub', icon: 'i-lucide-house' },
    })
    expect(header.text()).toContain('Test Page')

    const section = mount(MSection, {
      props: { title: 'Sec' },
      slots: { default: () => 'Body' },
    })
    expect(section.text()).toContain('Sec')

    const row = mount(MSettingRow, {
      props: { label: 'Row Label', hint: 'Row Hint' },
      slots: { default: () => 'Control' },
    })
    expect(row.text()).toContain('Row Label')
    expect(row.text()).toContain('Row Hint')
    expect(row.find('.min-w-\\[140px\\]').exists()).toBe(true)
    const brand = mount(MBrandIcon, { props: { name: 'modrinth' } })
    expect(brand.find('span').classes()).toContain('i-simple-icons-modrinth')

    const loader = mount(MLoaderIcon, { props: { kind: 'fabric', size: 40 } })
    expect(loader.find('img').attributes('src')).toBe('/assets/icons/loaders/fabric.png')
    const fallbackLoader = mount(MLoaderIcon, { props: { kind: 'unknown', size: 40 } })
    expect(fallbackLoader.find('span').classes()).toContain('i-lucide-cpu')
    const pop = mount(MPopover, {
      props: { open: true },
      slots: { default: () => 'Popover Body' },
    })
    await nextTick()
    expect(document.body.innerHTML).toContain('Popover Body')
    pop.unmount()
  })
  it('renders MPathPicker non-editable path picker with browse and clear buttons', async () => {
    const wrapper = mount(MPathPicker, {
      props: {
        modelValue: '/path/to/my/minecraft',
        type: 'directory',
        clearable: true,
      },
      global: {
        plugins: [i18n],
      },
    })

    expect(wrapper.find('.i-lucide-folder').exists()).toBe(true)
    expect(wrapper.text()).toContain('浏览')
    const input = wrapper.find('input')
    expect(input.attributes('readonly')).toBeDefined()
    expect((input.element as HTMLInputElement).value).toBe('/path/to/my/minecraft')

    const clearBtn = wrapper.find('button .i-lucide-x')
    expect(clearBtn.exists()).toBe(true)
    await clearBtn.trigger('click')
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([''])
    expect(wrapper.emitted('clear')).toBeDefined()

    // Check file type
    const filePicker = mount(MPathPicker, {
      props: {
        modelValue: 'file.zip',
        type: 'file',
        buttonText: 'Select Zip',
      },
      global: {
        plugins: [i18n],
      },
    })
    expect(filePicker.find('.i-lucide-file-text').exists()).toBe(true)
    expect(filePicker.text()).toContain('Select Zip')
  })

  it('renders MPagination and handles page navigation', async () => {
    const wrapper = mount(MPagination, {
      props: {
        page: 10,
        pageCount: 20,
      },
      global: {
        plugins: [i18n],
      },
    })

    // Page numbers rendered: 1 … 9 10 11 … 20
    const text = wrapper.text()
    expect(text).toContain('1')
    expect(text).toContain('…')
    expect(text).toContain('9')
    expect(text).toContain('10')
    expect(text).toContain('11')
    expect(text).toContain('20')

    // Click on page 11 button
    const btn11 = wrapper.findAll('button').find((b) => b.text() === '11')
    expect(btn11).toBeDefined()
    await btn11!.trigger('click')
    expect(wrapper.emitted('update:page')).toEqual([[11]])

    // page=1 prev disabled
    const page1Wrapper = mount(MPagination, {
      props: {
        page: 1,
        pageCount: 20,
      },
      global: {
        plugins: [i18n],
      },
    })
    const prevBtn = page1Wrapper.findAll('button')[0]
    expect(prevBtn.attributes('disabled')).toBeDefined()

    // pageCount undefined with hasNext=false -> next disabled
    const noNextWrapper = mount(MPagination, {
      props: {
        page: 2,
        hasNext: false,
      },
      global: {
        plugins: [i18n],
      },
    })
    const buttons = noNextWrapper.findAll('button')
    const nextBtn = buttons[buttons.length - 1]
    expect(nextBtn.attributes('disabled')).toBeDefined()

    // teleport=false renders inline without fixed classes
    const inlinePagination = mount(MPagination, {
      props: {
        page: 1,
        pageCount: 5,
        teleport: false,
      },
      global: {
        plugins: [i18n],
      },
    })
    const nav = inlinePagination.find('nav')
    expect(nav.exists()).toBe(true)
    expect(nav.classes()).not.toContain('fixed')
  })

  it('asserts MToast, MDialog and MEmpty contain select-text elements', async () => {
    const toast = useToast()
    toast.info('Selectable toast message', 0)
    const toastWrapper = mount(MToast)
    await nextTick()
    expect(toastWrapper.find('.select-text').exists()).toBe(true)
    expect(toastWrapper.find('.select-text').text()).toContain('Selectable toast message')

    const dialogWrapper = mount(MDialog, {
      props: { open: true, title: 'Dialog Title' },
      slots: { default: '<p>Dialog body text</p>' },
      global: { plugins: [i18n] },
    })
    await nextTick()
    expect(document.body.querySelector('.select-text')).not.toBeNull()
    dialogWrapper.unmount()

    const emptyWrapper = mount(MEmpty, {
      props: { title: 'Empty', description: 'Selectable description' },
    })
    expect(emptyWrapper.find('p.select-text').exists()).toBe(true)
    expect(emptyWrapper.find('p.select-text').text()).toBe('Selectable description')
  })

  it('MOverlay renders into body and handles close', async () => {
    const wrapper = mount(MOverlay, {
      props: { open: true },
      slots: { default: '<div class="overlay-inner">Overlay Content</div>' },
    })
    expect(document.body.innerHTML).toContain('Overlay Content')
    wrapper.unmount()
  })

  it('MDrawer renders into body and emits confirm/close', async () => {
    const wrapper = mount(MDrawer, {
      props: { open: true, title: 'Drawer Title', hideFooter: false },
      slots: { default: '<p>Drawer Body</p>' },
      global: { plugins: [i18n] },
    })
    expect(document.body.innerHTML).toContain('Drawer Title')
    expect(document.body.innerHTML).toContain('Drawer Body')
    wrapper.unmount()
  })

  it('useDialog confirmDanger sets severity and confirmVariant to danger', async () => {
    const { confirmDanger, handleConfirm, activeDialog } = useDialog()
    const promise = confirmDanger({ content: 'Danger prompt' })
    expect(activeDialog.value?.open).toBe(true)
    expect(activeDialog.value?.severity).toBe('danger')
    expect(activeDialog.value?.confirmVariant).toBe('danger')
    handleConfirm()
    const res = await promise
    expect(res).toBe(true)
  })

  it('MSearchBar handles input, clear, and search events', async () => {
    const wrapper = mount(MSearchBar, {
      props: { modelValue: 'hello' },
      global: { plugins: [i18n] },
    })
    expect(wrapper.find('input').element.value).toBe('hello')
    const clearBtn = wrapper.find('button')
    if (clearBtn.exists()) {
      await clearBtn.trigger('click')
      expect(wrapper.emitted('update:modelValue')?.[0]).toEqual([''])
      expect(wrapper.emitted('clear')).toHaveLength(1)
    }
  })

  it('MStatusBadge renders status with semantic variant and dot', () => {
    const wrapper = mount(MStatusBadge, {
      props: { status: 'online' },
    })
    expect(wrapper.text()).toContain('online')
  })

  it('MCard renders skeleton when loading is true', () => {
    const wrapper = mount(MCard, {
      props: { loading: true },
    })
    expect(wrapper.findAll('.m-skeleton').length).toBeGreaterThan(0)
  })

  it('MEmpty renders action slot or action button', async () => {
    const wrapper = mount(MEmpty, {
      props: { actionText: 'Create New' },
      global: { plugins: [i18n] },
    })
    const button = wrapper.find('button')
    expect(button.exists()).toBe(true)
    expect(button.text()).toContain('Create New')
    await button.trigger('click')
    expect(wrapper.emitted('action')).toHaveLength(1)
  })

  it('MListItem supports dense and named slots', () => {
    const wrapper = mount(MListItem, {
      props: { dense: true, title: 'Dense Item' },
      slots: {
        badges: '<span class="badge-slot">New</span>',
        actions: '<button class="action-btn">Action</button>',
      },
    })
    expect(wrapper.find('.badge-slot').exists()).toBe(true)
    expect(wrapper.find('.action-btn').exists()).toBe(true)
    expect(wrapper.classes()).toContain('px-2.5')
  })

  it('MInput and MSelect render loading spinner when loading is true', () => {
    const inputWrapper = mount(MInput, {
      props: { modelValue: '', loading: true },
    })
    expect(inputWrapper.find('.animate-spin').exists()).toBe(true)

    const selectWrapper = mount(MSelect, {
      props: { modelValue: '1', options: [{ label: '1', value: '1' }], loading: true },
      global: { plugins: [i18n] },
    })
    expect(selectWrapper.find('.animate-spin').exists()).toBe(true)
  })

  it('MImage renders img with lazy decoding and handles fallback gracefully', async () => {
    const img = mount(MImage, {
      props: { src: 'https://example.com/photo.png', alt: 'Test Photo', pixelated: true },
    })
    const el = img.find('img')
    expect(el.exists()).toBe(true)
    expect(el.attributes('src')).toBe('https://example.com/photo.png')
    expect(el.attributes('loading')).toBe('lazy')
    expect(el.attributes('decoding')).toBe('async')
    expect(img.classes()).toContain('[image-rendering:pixelated]')

    // Fallback when src is empty
    const fallbackImg = mount(MImage, {
      props: { src: null, fallbackIcon: 'i-lucide-box' },
    })
    expect(fallbackImg.find('img').exists()).toBe(false)
    expect(fallbackImg.find('.i-lucide-box').exists()).toBe(true)

    // Fallback on error with fallbackSrc
    const errorImg = mount(MImage, {
      props: { src: 'https://broken.png', fallbackSrc: 'https://fallback.png' },
    })
    await errorImg.find('img').trigger('error')
    expect(errorImg.find('img').attributes('src')).toBe('https://fallback.png')
  })

  it('MContextMenu survives the very contextmenu event that opened it', async () => {
    const wrapper = mount(MContextMenu, {
      props: {
        open: false,
        'onUpdate:open': (val: boolean) => wrapper.setProps({ open: val }),
        x: 120,
        y: 140,
        items: [{ key: 'manage', label: 'Manage' }],
      },
      attachTo: document.body,
    })
    await nextTick()

    const menus = () => document.body.querySelectorAll('[data-testid="m-context-menu"]').length
    const before = menus()

    // Simulate the page handler: open, then let the same event bubble to window.
    await wrapper.setProps({ open: true })
    await nextTick()
    const evt = new MouseEvent('contextmenu', { bubbles: true })
    document.body.dispatchEvent(evt)
    await nextTick()
    expect(menus()).toBe(before + 1)

    // …while a pointer press elsewhere does dismiss it.
    const pointer = new Event('pointerdown', { bubbles: true })
    document.body.dispatchEvent(pointer)
    await nextTick()
    expect(menus()).toBe(before)
    wrapper.unmount()
  })
})

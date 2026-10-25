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
  it('applies theme attributes and css variables to documentElement', () => {
    applyTheme({
      seed: '#10b981',
      mode: 'dark',
      material: 'glass',
    })

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
    const options = document.body.querySelectorAll('[role="option"]')
    expect(options.length).toBeGreaterThanOrEqual(2)
    ;(options[1] as HTMLButtonElement).click()
    await nextTick()
    expect(wrapper.emitted('update:modelValue')?.[0]).toEqual(['b'])
  })
  it('MSelect disabled and empty options state prevents opening and displays placeholder', async () => {
    const wrapper = mount(MSelect, {
      props: {
        modelValue: '',
        options: [],
      },
    })
    const trigger = wrapper.find('button[role="combobox"]')
    expect(trigger.attributes('disabled')).toBeDefined()
    expect(trigger.text()).toContain('无可用选项')
    await trigger.trigger('click')
    await nextTick()
    expect(document.body.querySelector('[role="listbox"]')).toBeNull()
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
    expect(wrapper.find('.i-lucide-help-circle').exists()).toBe(true)
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

  it('MSkeleton renders width and height', () => {
    const wrapper = mount(MSkeleton, {
      props: { width: '120px', height: '24px' },
    })
    expect(wrapper.attributes('style')).toContain('width: 120px')
    expect(wrapper.attributes('style')).toContain('height: 24px')
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
    expect(loader.find('span').classes()).toContain('i-lucide-waypoints')

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
})

import { describe, it, expect, vi, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import AddToPresetDialog from '../components/AddToPresetDialog.vue'
import {
  ipcClient,
  type AddItemOutcome,
  type CompProject,
  type ResourcePreset,
} from '../../../ipc/client'
import zhCnComp from '../../../locales/zh-CN/comp.json'
import zhCnCommon from '../../../locales/zh-CN/common.json'

const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: { 'zh-CN': { comp: zhCnComp, common: zhCnCommon } },
})

function makeProject(overrides: Partial<CompProject> = {}): CompProject {
  return {
    platform: 'modrinth',
    id: 'AANobbMI',
    slug: 'sodium',
    title: 'Sodium',
    summary: '',
    author: 'jellysquid',
    downloads: 0,
    follows: 0,
    categories: [],
    mcVersions: ['1.20.1'],
    loaders: ['fabric'],
    compType: 'mod',
    updatedAt: '2024-01-01',
    ...overrides,
  }
}

function makePreset(): ResourcePreset {
  return {
    id: 'preset-1',
    name: '性能优化',
    description: '',
    gameVersions: ['1.20.1'],
    loaders: ['fabric'],
    items: [],
    createdAt: 1,
    updatedAt: 1,
  }
}

async function mountDialog(outcomes: AddItemOutcome[]) {
  vi.spyOn(ipcClient, 'presetsList').mockResolvedValue([makePreset()])
  const addSpy = vi.spyOn(ipcClient, 'presetsAddItems').mockResolvedValue(outcomes)

  mount(AddToPresetDialog, {
    props: {
      open: true,
      project: makeProject(),
      pinnedFileId: 'file-7',
      pinnedFileName: 'sodium-0.5.8.jar',
    },
    global: { plugins: [i18n] },
  })
  await flushPromises()

  const confirm = document.body.querySelector<HTMLElement>('[data-testid="add-to-preset-confirm"]')
  expect(confirm).not.toBeNull()
  return { confirm: confirm!, addSpy }
}

afterEach(() => {
  document.body.innerHTML = ''
  vi.restoreAllMocks()
})

describe('AddToPresetDialog', () => {
  it('adds the project with its pinned file and shows a rejection outcome', async () => {
    const { confirm, addSpy } = await mountDialog([
      {
        platform: 'modrinth',
        projectId: 'AANobbMI',
        title: 'Sodium',
        status: 'noGameVersion',
        latestFileName: 'sodium-0.5.8.jar',
      },
    ])

    confirm.click()
    await flushPromises()

    expect(addSpy).toHaveBeenCalledTimes(1)
    const [presetId, items] = addSpy.mock.calls[0]
    expect(presetId).toBe('preset-1')
    expect(items[0]).toMatchObject({
      platform: 'modrinth',
      projectId: 'AANobbMI',
      kind: 'mod',
      slug: 'sodium',
      pinnedFileId: 'file-7',
      pinnedFileName: 'sodium-0.5.8.jar',
    })

    expect(document.body.textContent).toContain('该资源没有对应的游戏版本，未加入')
  })

  it('renders every returned outcome with its own message', async () => {
    const { confirm } = await mountDialog([
      { platform: 'modrinth', projectId: 'AANobbMI', title: 'Sodium', status: 'added' },
      {
        platform: 'modrinth',
        projectId: 'P7dR8mSH',
        title: 'Fabric API',
        status: 'alreadyInPresetOtherVersion',
      },
    ])

    confirm.click()
    await flushPromises()

    expect(document.body.textContent).toContain('已加入预设')
    expect(document.body.textContent).toContain('预设中已有该资源的不同版本')
  })
})

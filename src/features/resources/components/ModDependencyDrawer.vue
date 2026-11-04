<script setup lang="ts">
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import type { DepItem, ModDependencyReport, ResourceEntry } from '../../../ipc/client'
import { ipcClient } from '../../../ipc/client'
import { MDrawer, MBadge, MButton, MEmpty } from '../../../ui'

const props = defineProps<{
  open: boolean
  entry: ResourceEntry | null
  /** Backend analysis for the mod, or null when the scan has not completed yet. */
  report: ModDependencyReport | null
}>()

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void
  (e: 'jump', fileName: string): void
  (e: 'enable', fileName: string): void
}>()

const { t } = useI18n()
const router = useRouter()
const lookupId = ref<string | null>(null)

const subtitle = computed(() => {
  const name = props.entry?.title || props.entry?.fileName || ''
  return name ? t('resources.dependencyDrawer.subtitle', { name }) : undefined
})

const deps = computed<DepItem[]>(() => props.report?.deps ?? [])

const dependents = computed<string[]>(() => {
  if (props.report) return props.report.dependents
  return props.entry?.dependents ?? []
})

function statusVariant(dep: DepItem): 'success' | 'warning' | 'info' | 'danger' | 'neutral' {
  if (dep.kind === 'optional' && dep.status === 'missing') {
    return 'neutral'
  }
  switch (dep.status) {
    case 'installed':
      return 'success'
    case 'disabled':
    case 'versionMismatch':
      return 'warning'
    case 'bundled':
      return 'info'
    case 'missing':
    case 'conflict':
      return 'danger'
    default:
      return 'neutral'
  }
}

function statusText(dep: DepItem): string {
  if (dep.kind === 'optional' && dep.status === 'missing') {
    return t('resources.dependencyDrawer.status.optionalMissing')
  }
  return t(`resources.dependencyDrawer.status.${dep.status}`)
}

function kindText(kind: DepItem['kind']): string {
  return t(`resources.dependencyDrawer.kind.${kind}`)
}

function rangeText(dep: DepItem): string {
  return dep.range
    ? t('resources.dependencyDrawer.range', { range: dep.range })
    : t('resources.dependencyDrawer.anyRange')
}

/** Provider rows can be located in the loaded list; a missing id must be looked up online. */
function jumpable(dep: DepItem): boolean {
  return (
    Boolean(dep.providerFile) &&
    (dep.status === 'installed' ||
      dep.status === 'disabled' ||
      dep.status === 'versionMismatch' ||
      dep.status === 'conflict')
  )
}

function jumpTo(fileName: string) {
  emit('update:open', false)
  emit('jump', fileName)
}

function enableProvider(fileName: string) {
  emit('update:open', false)
  emit('enable', fileName)
}

async function lookupMissing(dep: DepItem) {
  if (lookupId.value) return
  lookupId.value = dep.id
  try {
    const project = await ipcClient.compDepLookup(dep.id)
    emit('update:open', false)
    if (project) {
      await router.push(`/comp/project/${project.platform}/${project.id}`)
    } else {
      await router.push({ path: '/comp', query: { q: dep.id } })
    }
  } catch {
    // The lookup is a convenience: a failed request must not trap the drawer.
  } finally {
    lookupId.value = null
  }
}

function onDependencyClick(dep: DepItem) {
  if (dep.status === 'missing') {
    void lookupMissing(dep)
    return
  }
  if (jumpable(dep) && dep.providerFile) jumpTo(dep.providerFile)
}
</script>

<template>
  <MDrawer
    :open="open"
    size="md"
    :title="t('resources.dependencyDrawer.title')"
    :subtitle="subtitle"
    @update:open="emit('update:open', $event)"
  >
    <div class="space-y-5">
      <section class="space-y-2">
        <h4 class="text-xs font-semibold text-fg-muted uppercase tracking-wide">
          {{ t('resources.dependencyDrawer.depsTitle') }}
        </h4>

        <MEmpty
          v-if="deps.length === 0"
          :title="t('resources.dependencyDrawer.empty')"
          class="py-6"
        />

        <div v-else class="space-y-1.5" data-testid="dependency-list">
          <div
            v-for="dep in deps"
            :key="dep.id + '-' + dep.kind"
            class="p-2.5 rounded-xl border border-line bg-panel transition"
            :class="jumpable(dep) || dep.status === 'missing' ? 'hover:border-line-strong' : ''"
            data-testid="dependency-row"
          >
            <div class="flex items-center gap-2">
              <button
                type="button"
                class="min-w-0 flex-1 text-left"
                :class="
                  jumpable(dep) || dep.status === 'missing' ? 'cursor-pointer' : 'cursor-default'
                "
                :data-testid="'dependency-main-' + dep.id"
                @click="onDependencyClick(dep)"
              >
                <div class="text-sm font-medium text-fg truncate">{{ dep.id }}</div>
                <div class="text-[11px] text-fg-muted truncate">
                  {{ kindText(dep.kind) }} · {{ rangeText(dep) }}
                </div>
              </button>

              <MBadge :variant="statusVariant(dep)" size="sm">
                {{ statusText(dep) }}
              </MBadge>

              <MButton
                v-if="dep.status === 'disabled' && dep.providerFile"
                variant="tonal"
                size="sm"
                data-testid="dependency-enable"
                @click.stop="enableProvider(dep.providerFile)"
              >
                {{ t('resources.dependencyDrawer.enable') }}
              </MButton>
              <MButton
                v-if="dep.status === 'missing'"
                variant="tonal"
                size="sm"
                :loading="lookupId === dep.id"
                @click.stop="lookupMissing(dep)"
              >
                {{ t('resources.dependencyDrawer.lookup') }}
              </MButton>
            </div>

            <div
              v-if="dep.providerFile"
              class="text-[11px] text-fg-muted mt-1 truncate"
              :title="dep.providerFile"
            >
              {{
                dep.providerVersion
                  ? t('resources.dependencyDrawer.providerWithVersion', {
                      file: dep.providerFile,
                      version: dep.providerVersion,
                    })
                  : t('resources.dependencyDrawer.provider', { file: dep.providerFile })
              }}
            </div>
          </div>
        </div>
      </section>

      <section class="space-y-2">
        <h4 class="text-xs font-semibold text-fg-muted uppercase tracking-wide">
          {{ t('resources.dependencyDrawer.dependentsTitle') }}
        </h4>

        <MEmpty
          v-if="dependents.length === 0"
          :title="t('resources.dependencyDrawer.emptyDependents')"
          class="py-6"
        />

        <div v-else class="space-y-1.5" data-testid="dependent-list">
          <button
            v-for="name in dependents"
            :key="name"
            type="button"
            class="w-full flex items-center gap-2 p-2 rounded-xl border border-line bg-panel hover:border-line-strong text-left transition cursor-pointer"
            data-testid="dependent-row"
            @click="jumpTo(name)"
          >
            <span class="i-lucide-arrow-up-right w-3.5 h-3.5 text-primary shrink-0" />
            <span class="text-xs text-fg truncate flex-1 min-w-0">{{ name }}</span>
          </button>
        </div>
      </section>
    </div>
  </MDrawer>
</template>

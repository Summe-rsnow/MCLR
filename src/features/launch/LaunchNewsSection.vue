<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { openUrl } from '@tauri-apps/plugin-opener'
import { useHomeStore } from '../home/store'
import {
  MCard,
  MButton,
  MIconButton,
  MSwitch,
  MEmpty,
  MSkeleton,
  MPagination,
  MDialog,
  MInput,
  MBadge,
  MImage,
} from '../../ui'

defineProps<{
  showNews: boolean
}>()

const emit = defineEmits<{
  'update:showNews': [val: boolean]
  refresh: [force: boolean]
}>()

const { t } = useI18n()
const router = useRouter()
const homeStore = useHomeStore()

const showJumpDialog = ref(false)
const jumpPageInput = ref('')

async function openExternal(url?: string | null) {
  if (!url) return
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}

function handleAnnouncementAction(url?: string | null) {
  if (!url) {
    router.push('/settings/about')
    return
  }
  openExternal(url)
}

function openJumpDialog() {
  jumpPageInput.value = String(homeStore.page || 1)
  showJumpDialog.value = true
}

function onConfirmJump() {
  const target = parseInt(jumpPageInput.value, 10)
  if (!Number.isNaN(target) && target >= 1) {
    homeStore.loadNews(target)
  }
  showJumpDialog.value = false
}
</script>

<template>
  <div class="space-y-4 pt-2">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-2">
        <span class="i-lucide-newspaper w-5 h-5 text-primary" />
        <h3 class="text-base font-semibold text-fg">
          {{ t('launch.news.title') }}
        </h3>
      </div>
      <div class="flex items-center gap-3">
        <MButton
          v-if="showNews"
          data-testid="news-refresh-btn"
          size="xs"
          variant="tonal"
          icon="i-lucide-refresh-cw"
          :loading="homeStore.loadingNews"
          @click="emit('refresh', true)"
        >
          {{ homeStore.loadingNews ? t('launch.news.refreshing') : t('launch.news.refresh') }}
        </MButton>
        <div class="flex items-center gap-2">
          <span class="text-xs text-fg-muted select-none">
            {{ t('launch.news.toggleLabel') }}
          </span>
          <MSwitch
            :model-value="showNews"
            data-testid="news-toggle-switch"
            size="sm"
            @update:model-value="emit('update:showNews', $event)"
          />
        </div>
      </div>
    </div>

    <!-- Off State: Compact Card -->
    <MCard
      v-if="!showNews"
      data-testid="news-compact-off-card"
      variant="well"
      padding="md"
      class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4"
    >
      <div class="flex items-center gap-3 min-w-0">
        <div
          class="w-9 h-9 rounded-xl bg-sunken flex items-center justify-center shrink-0 text-fg-muted"
        >
          <span class="i-lucide-newspaper w-4.5 h-4.5" />
        </div>
        <div class="min-w-0">
          <p class="text-xs text-fg-muted">
            {{ t('launch.news.disabledHint') }}
          </p>
        </div>
      </div>
      <MButton
        data-testid="news-enable-btn"
        size="xs"
        variant="tonal"
        @click="emit('update:showNews', true)"
      >
        {{ t('launch.news.enableAction') }}
      </MButton>
    </MCard>

    <!-- On State: Announcements & News List -->
    <div v-else class="space-y-4" data-testid="news-container-on">
      <!-- Announcements Banner -->
      <div v-if="homeStore.announcements.length > 0" class="flex flex-col gap-2">
        <div
          v-for="item in homeStore.announcements"
          :key="item.id"
          class="p-4 rounded-xl border border-primary/20 bg-primary-soft text-primary-soft-fg flex flex-col md:flex-row md:items-center justify-between gap-3"
        >
          <div class="flex items-start gap-3">
            <span class="i-lucide-megaphone w-5 h-5 text-primary shrink-0 mt-0.5" />
            <div class="flex flex-col gap-1">
              <span class="text-sm font-semibold text-fg">{{ item.title }}</span>
              <p class="text-xs text-fg-muted leading-relaxed">{{ item.detail }}</p>
            </div>
          </div>
          <div class="flex items-center gap-2 shrink-0 self-end md:self-auto">
            <MButton
              v-if="item.btn1"
              size="sm"
              variant="tonal"
              @click="handleAnnouncementAction(item.btn1.url)"
            >
              {{ item.btn1.text }}
            </MButton>
            <MButton
              v-if="item.btn2"
              size="sm"
              variant="tonal"
              @click="handleAnnouncementAction(item.btn2.url)"
            >
              {{ item.btn2.text }}
            </MButton>
            <MIconButton
              icon="i-lucide-x"
              size="sm"
              variant="text"
              :tooltip="t('launch.announcements.dismiss')"
              @click="homeStore.dismissAnnouncement(item.id)"
            />
          </div>
        </div>
      </div>

      <!-- News Loading Skeleton -->
      <div
        v-if="homeStore.loadingNews && homeStore.news.length === 0"
        class="columns-1 md:columns-2 lg:columns-3 gap-4"
        data-testid="news-skeleton"
      >
        <MCard
          v-for="n in 6"
          :key="n"
          class="break-inside-avoid mb-4 overflow-hidden flex flex-col"
        >
          <MSkeleton height="9rem" class="w-full aspect-video rounded-none" :rounded="false" />
          <div class="p-4 flex-1 flex flex-col justify-between gap-2">
            <div class="flex flex-col gap-1.5">
              <MSkeleton width="33%" height="0.75rem" />
              <MSkeleton :width="n % 2 === 0 ? '85%' : '70%'" height="1rem" class="mt-1" />
              <div class="space-y-1.5 mt-2">
                <MSkeleton width="100%" height="0.75rem" />
                <MSkeleton
                  :width="n % 3 === 0 ? '90%' : n % 3 === 1 ? '75%' : '60%'"
                  height="0.75rem"
                />
              </div>
            </div>
          </div>
        </MCard>
      </div>

      <!-- News Error -->
      <div
        v-else-if="homeStore.newsError && homeStore.news.length === 0"
        class="py-8 flex items-center justify-center"
      >
        <MEmpty :description="t('launch.news.fetchFailed')">
          <MButton size="sm" @click="emit('refresh', true)">
            {{ t('launch.news.refresh') }}
          </MButton>
        </MEmpty>
      </div>

      <!-- News Empty on page > 1 -->
      <div
        v-else-if="homeStore.news.length === 0 && homeStore.page > 1"
        class="py-8 flex items-center justify-center"
      >
        <MEmpty
          :title="t('launch.news.emptyPageTitle')"
          :description="t('launch.news.emptyPageDesc')"
        >
          <MButton variant="primary" size="sm" @click="homeStore.loadNews(1)">
            {{ t('launch.news.backToFirstPage') }}
          </MButton>
        </MEmpty>
      </div>

      <!-- News Empty -->
      <div v-else-if="homeStore.news.length === 0" class="py-8 flex items-center justify-center">
        <MEmpty :description="t('launch.news.empty')" />
      </div>

      <!-- News Grid -->
      <div v-else class="space-y-4 pb-24">
        <div class="columns-1 md:columns-2 lg:columns-3 gap-4">
          <MCard
            v-for="(item, index) in homeStore.news"
            :key="item.id"
            hoverable
            class="break-inside-avoid mb-4 flex flex-col cursor-pointer overflow-hidden group m-stagger"
            :style="{ '--i': index }"
            @click="openExternal(item.articleUrl)"
          >
            <!-- Cover Image -->
            <div class="w-full h-36 bg-sunken relative overflow-hidden shrink-0">
              <MImage
                :src="item.imageUrl"
                alt="cover"
                fallback-icon="i-lucide-newspaper"
                class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
              />
              <div class="absolute top-2 left-2">
                <MBadge variant="primary">{{ item.category }}</MBadge>
              </div>
            </div>

            <!-- Content -->
            <div class="p-4 flex-1 flex flex-col justify-between gap-2">
              <div class="flex flex-col gap-1.5">
                <div class="flex items-center justify-between text-xs text-fg-muted">
                  <span>{{ item.date }}</span>
                </div>
                <h4
                  class="text-sm font-semibold text-fg line-clamp-2 leading-snug group-hover:text-primary transition-colors"
                >
                  {{ item.title }}
                </h4>
                <p class="text-xs text-fg-muted line-clamp-3 leading-relaxed">
                  {{ item.summary }}
                </p>
              </div>

              <div class="pt-2 flex items-center text-xs text-primary font-medium gap-1 self-end">
                <span>{{ t('launch.news.readMore') }}</span>
                <span class="i-lucide-arrow-right w-3.5 h-3.5" />
              </div>
            </div>
          </MCard>
        </div>

        <!-- Floating Pagination and Jump -->
        <div class="flex items-center justify-center gap-2">
          <MPagination
            :page="homeStore.page"
            :has-next="homeStore.news.length > 0"
            :loading="homeStore.loadingNews"
            @update:page="(p: number) => homeStore.loadNews(p)"
          />
          <div class="fixed bottom-4 right-6 z-30 pointer-events-auto">
            <MButton
              variant="tonal"
              size="xs"
              icon="i-lucide-circle-arrow-right"
              class="m-overlay m-refract rounded-full shadow-2"
              data-testid="news-jump-btn"
              @click="openJumpDialog"
            >
              {{ t('launch.news.jumpPage') }}
            </MButton>
          </div>
        </div>
      </div>
    </div>

    <!-- Jump Page Dialog -->
    <MDialog
      :open="showJumpDialog"
      :title="t('launch.news.jumpPage')"
      :confirm-text="t('common.confirm')"
      :cancel-text="t('common.cancel')"
      @update:open="showJumpDialog = $event"
      @confirm="onConfirmJump"
      @cancel="showJumpDialog = false"
    >
      <div class="space-y-3 py-2">
        <label class="text-xs text-fg-muted">{{ t('launch.news.jumpPagePrompt') }}</label>
        <MInput
          v-model="jumpPageInput"
          type="number"
          :min="1"
          autofocus
          data-testid="jump-page-input"
          @keydown.enter="onConfirmJump"
        />
      </div>
    </MDialog>
  </div>
</template>

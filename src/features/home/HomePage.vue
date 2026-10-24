<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { openUrl } from '@tauri-apps/plugin-opener'
import {
  MCard,
  MButton,
  MIconButton,
  MInput,
  MBadge,
  MEmpty,
  MSkeleton,
  MScroll,
  MPageHeader,
  MSegmented,
} from '../../ui'
import { useHomeStore } from './store'
import helpIndexRaw from '../../help/zh-CN/index.json'
interface HelpArticleMeta {
  id: string
  title: string
  category: string
  tags: string[]
}

const helpArticles = helpIndexRaw as HelpArticleMeta[]

const helpDocsMap = import.meta.glob<string>('../../help/zh-CN/*.md', {
  query: '?raw',
  import: 'default',
  eager: true,
})

const md = new MarkdownIt({
  html: false,
  linkify: true,
  breaks: true,
})

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const homeStore = useHomeStore()

const activeTab = ref<'news' | 'help'>('news')
const selectedCategory = ref<string>('all')

const categories = computed(() => {
  const set = new Set<string>()
  for (const doc of helpArticles) {
    if (doc.category) {
      set.add(doc.category)
    }
  }
  return ['all', ...Array.from(set)]
})

const filteredArticles = computed(() => {
  const query = homeStore.helpQuery.trim().toLowerCase()
  return helpArticles.filter((doc) => {
    if (selectedCategory.value !== 'all' && doc.category !== selectedCategory.value) {
      return false
    }
    if (!query) {
      return true
    }
    const matchTitle = doc.title.toLowerCase().includes(query)
    const matchCategory = doc.category.toLowerCase().includes(query)
    const matchTags = doc.tags.some((tag) => tag.toLowerCase().includes(query))
    return matchTitle || matchCategory || matchTags
  })
})

const currentHelpMarkdown = computed(() => {
  const targetId = homeStore.selectedHelpId
  const key = `../../help/zh-CN/${targetId}.md`
  return helpDocsMap[key] || ''
})

const renderedHelpHtml = computed(() => {
  if (!currentHelpMarkdown.value) {
    return ''
  }
  const rawHtml = md.render(currentHelpMarkdown.value)
  return DOMPurify.sanitize(rawHtml)
})

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
    activeTab.value = 'help'
    router.replace({ path: '/home', query: { tab: 'help' } })
    return
  }
  openExternal(url)
}

function changePage(delta: number) {
  const next = homeStore.page + delta
  if (next >= 1) {
    homeStore.loadNews(next, false)
  }
}

watch(
  () => route.query.tab,
  (tab) => {
    if (tab === 'help' || tab === 'news') {
      activeTab.value = tab
    }
  },
  { immediate: true },
)

watch(activeTab, (tab) => {
  if (route.query.tab !== tab) {
    router.replace({ path: '/home', query: { ...route.query, tab } })
  }
})

onMounted(() => {
  homeStore.loadAnnouncements()
  if (homeStore.news.length === 0) {
    homeStore.loadNews(1, false)
  }
})
</script>

<template>
  <div class="m-page">
    <MPageHeader
      :title="t('home.title')"
      :description="t('home.subtitle')"
      icon="i-lucide-house"
    >
      <template #actions>
        <div class="flex items-center gap-3">
          <MSegmented
            v-model="activeTab"
            :options="[
              { label: t('home.tabs.news'), value: 'news' },
              { label: t('home.tabs.help'), value: 'help' },
            ]"
            size="sm"
          />
          <MButton
            v-if="activeTab === 'news'"
            size="sm"
            variant="tonal"
            icon="i-lucide-refresh-cw"
            :loading="homeStore.loadingNews"
            @click="homeStore.loadNews(homeStore.page, true)"
          >
            {{ homeStore.loadingNews ? t('home.news.refreshing') : t('home.news.refresh') }}
          </MButton>
        </div>
      </template>
    </MPageHeader>

    <!-- Tab 1: News & Announcements -->
    <div v-if="activeTab === 'news'" class="flex-1 flex flex-col gap-4 overflow-hidden">
      <!-- Announcements Banner -->
      <div v-if="homeStore.announcements.length > 0" class="flex flex-col gap-2 shrink-0">
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
              :tooltip="t('home.announcements.dismiss')"
              @click="homeStore.dismissAnnouncement(item.id)"
            />
          </div>
        </div>
      </div>

      <!-- News Container -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <div v-if="homeStore.loadingNews && homeStore.news.length === 0" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 p-1">
          <div v-for="i in 6" :key="i" class="p-4 rounded-xl border border-line flex flex-col gap-3">
            <MSkeleton class="w-full h-36 rounded-lg" />
            <MSkeleton class="w-2/3 h-5 rounded" />
            <MSkeleton class="w-full h-12 rounded" />
          </div>
        </div>

        <div v-else-if="homeStore.newsError && homeStore.news.length === 0" class="flex-1 flex items-center justify-center">
          <MEmpty :description="t('home.news.fetchFailed')">
            <MButton size="sm" @click="homeStore.loadNews(homeStore.page, true)">
              {{ t('home.news.refresh') }}
            </MButton>
          </MEmpty>
        </div>

        <div v-else-if="homeStore.news.length === 0" class="flex-1 flex items-center justify-center">
          <MEmpty :description="t('home.news.empty')" />
        </div>

        <MScroll v-else class="flex-1 pr-1">
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 pb-4">
            <MCard
              v-for="item in homeStore.news"
              :key="item.id"
              hoverable
              class="flex flex-col h-full cursor-pointer overflow-hidden group"
              @click="openExternal(item.articleUrl)"
            >
              <!-- Cover Image -->
              <div class="w-full h-40 bg-sunken relative overflow-hidden shrink-0">
                <img
                  v-if="item.imageUrl"
                  :src="item.imageUrl"
                  alt="cover"
                  class="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
                  loading="lazy"
                />
                <div v-else class="w-full h-full flex items-center justify-center text-fg-muted/40">
                  <span class="i-lucide-newspaper w-12 h-12" />
                </div>
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
                  <h3 class="text-sm font-semibold text-fg line-clamp-2 leading-snug group-hover:text-primary transition-colors">
                    {{ item.title }}
                  </h3>
                  <p class="text-xs text-fg-muted line-clamp-3 leading-relaxed">
                    {{ item.summary }}
                  </p>
                </div>

                <div class="pt-2 flex items-center text-xs text-primary font-medium gap-1 self-end">
                  <span>{{ t('home.news.readMore') }}</span>
                  <span class="i-lucide-arrow-right w-3.5 h-3.5" />
                </div>
              </div>
            </MCard>
          </div>
        </MScroll>

        <!-- Pagination Bar -->
        <div class="flex items-center justify-between pt-3 border-t border-line shrink-0">
          <MButton
            size="sm"
            variant="tonal"
            :disabled="homeStore.page <= 1 || homeStore.loadingNews"
            @click="changePage(-1)"
          >
            {{ t('home.news.pagePrev') }}
          </MButton>
          <span class="text-xs text-fg-muted">
            {{ t('home.news.pageNumber', { page: homeStore.page }) }}
          </span>
          <MButton
            size="sm"
            variant="tonal"
            :disabled="homeStore.news.length === 0 || homeStore.loadingNews"
            @click="changePage(1)"
          >
            {{ t('home.news.pageNext') }}
          </MButton>
        </div>
      </div>
    </div>

    <!-- Tab 2: Help Center -->
    <div v-else class="flex-1 flex flex-col md:flex-row gap-4 overflow-hidden">
      <!-- Left sidebar: Search & Article list -->
      <div class="w-full md:w-80 flex flex-col gap-3 shrink-0 border-b md:border-b-0 md:border-r border-line pb-4 md:pb-0 md:pr-4 overflow-hidden">
        <MInput
          v-model="homeStore.helpQuery"
          :placeholder="t('home.help.searchPlaceholder')"
          clearable
        >
          <template #prefix>
            <span class="i-lucide-search w-4 h-4 text-fg-muted" />
          </template>
        </MInput>

        <!-- Category Pills -->
        <div class="flex items-center gap-1.5 overflow-x-auto pb-1 shrink-0 scrollbar-none">
          <button
            v-for="cat in categories"
            :key="cat"
            type="button"
            class="px-2.5 py-1 rounded-full text-xs font-medium whitespace-nowrap transition-colors"
            :class="
              selectedCategory === cat
                ? 'bg-primary text-primary-on'
                : 'bg-sunken text-fg-muted hover:text-fg'
            "
            @click="selectedCategory = cat"
          >
            {{ cat === 'all' ? t('home.help.categories.all') : cat }}
          </button>
        </div>

        <!-- Article List -->
        <div v-if="filteredArticles.length === 0" class="flex-1 flex items-center justify-center p-4">
          <MEmpty :description="t('home.help.emptyArticles')" />
        </div>
        <MScroll v-else class="flex-1 pr-1">
          <div class="flex flex-col gap-2">
            <button
              v-for="doc in filteredArticles"
              :key="doc.id"
              data-test="help-article-item"
              type="button"
              class="w-full text-left p-3 rounded-xl border transition-all flex flex-col gap-1.5"
              :class="
                homeStore.selectedHelpId === doc.id
                  ? 'border-primary bg-primary-soft text-primary-soft-fg shadow-xs'
                  : 'border-line hover:border-line hover:bg-sunken text-fg'
              "
              @click="homeStore.selectedHelpId = doc.id"
            >
              <div class="flex items-center justify-between gap-2">
                <span class="text-xs font-semibold line-clamp-1">{{ doc.title }}</span>
                <MBadge variant="neutral">{{ doc.category }}</MBadge>
              </div>
              <div class="flex items-center gap-1 flex-wrap">
                <span
                  v-for="tag in doc.tags"
                  :key="tag"
                  class="text-[10px] text-fg-muted bg-raised px-1.5 py-0.5 rounded"
                >
                  #{{ tag }}
                </span>
              </div>
            </button>
          </div>
        </MScroll>
      </div>

      <!-- Right Viewer: Markdown Reader -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <div v-if="!currentHelpMarkdown" class="flex-1 flex items-center justify-center">
          <MEmpty :description="t('home.help.selectPrompt')" />
        </div>
        <MScroll v-else class="flex-1 pr-2">
          <div
            class="prose prose-sm dark:prose-invert max-w-none p-5 rounded-xl m-card leading-relaxed"
            v-html="renderedHelpHtml"
          />
        </MScroll>
      </div>
    </div>
  </div>
</template>

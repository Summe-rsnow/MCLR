<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { openUrl } from '@tauri-apps/plugin-opener'
import { MCard, MSection, MButton, MBadge, MCollapse } from '../../ui'

const { t } = useI18n()

import pkg from '../../../package.json'
const appVersion = `v${pkg.version}`
const repoUrl = 'https://github.com/mclr-dev/mclr'
const issuesUrl = 'https://github.com/mclr-dev/mclr/issues'

async function openExternal(url: string) {
  try {
    await openUrl(url)
  } catch {
    window.open(url, '_blank')
  }
}

// Collapsible FAQ states (account expanded by default)
const openFaqs = ref<Record<string, boolean>>({
  account: true,
  mods: false,
  java: false,
  crash: false,
})

function toggleFaq(id: string) {
  openFaqs.value[id] = !openFaqs.value[id]
}
</script>

<template>
  <div class="space-y-6 pb-12 select-none" data-testid="settings-about-page">
    <!-- App Overview Card -->
    <MCard variant="tinted" padding="lg" class="relative overflow-hidden">
      <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-6">
        <div class="flex items-center gap-4">
          <div
            class="w-14 h-14 rounded-2xl bg-primary flex items-center justify-center text-primary-on shadow-2 shrink-0"
          >
            <span class="i-lucide-rocket w-8 h-8" />
          </div>
          <div class="space-y-1">
            <div class="flex items-center gap-2.5 select-text">
              <h2 class="text-xl font-bold text-fg">MCLR</h2>
              <MBadge variant="primary">{{ appVersion }}</MBadge>
              <MBadge variant="neutral">Apache-2.0</MBadge>
            </div>
            <p class="text-xs text-fg-muted max-w-lg leading-relaxed">
              {{ t('settings.about.description') }}
            </p>
          </div>
        </div>

        <div class="flex items-center gap-2.5 shrink-0 w-full sm:w-auto">
          <MButton variant="tonal" size="sm" icon="i-lucide-github" @click="openExternal(repoUrl)">
            {{ t('settings.about.githubRepo') }}
          </MButton>
        </div>
      </div>
    </MCard>

    <!-- Feedback Section -->
    <MSection
      :title="t('settings.about.feedback.title')"
      :description="t('settings.about.feedback.subtitle')"
      icon="i-lucide-message-square"
    >
      <div class="flex flex-col gap-4">
        <div class="text-xs text-fg-muted select-text">
          <span class="font-medium text-fg block mb-2">{{
            t('settings.about.feedback.guideTitle')
          }}</span>
          <ul class="list-disc list-inside space-y-1">
            <li>{{ t('settings.about.feedback.guide1') }}</li>
            <li>{{ t('settings.about.feedback.guide2') }}</li>
            <li>{{ t('settings.about.feedback.guide3') }}</li>
          </ul>
        </div>

        <div>
          <MButton
            variant="primary"
            size="sm"
            icon="i-lucide-external-link"
            data-testid="open-github-btn"
            @click="openExternal(issuesUrl)"
          >
            {{ t('settings.about.feedback.openGithub') }}
          </MButton>
        </div>
      </div>
    </MSection>

    <!-- FAQ & Help Guide Section -->
    <MSection
      :title="t('settings.about.faqTitle')"
      :description="t('settings.about.faqSubtitle')"
      icon="i-lucide-circle-question-mark"
    >
      <div class="space-y-3">
        <!-- 1. Account -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-account"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('account')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div
                class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
              >
                <span class="i-lucide-user-round w-4 h-4" />
              </div>
              <div class="min-w-0">
                <h4 class="text-sm font-semibold text-fg">
                  {{ t('settings.about.questions.account.title') }}
                </h4>
                <p class="text-xs text-fg-muted truncate mt-0.5">
                  {{ t('settings.about.questions.account.desc') }}
                </p>
              </div>
            </div>
            <span
              class="i-lucide-chevron-down w-4 h-4 text-fg-muted transition-transform duration-200 shrink-0 ml-3"
              :class="{ 'rotate-180': openFaqs.account }"
            />
          </button>
          <MCollapse :open="openFaqs.account">
            <div
              data-testid="faq-content-account"
              class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50 select-text"
            >
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.account.msTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.account.msDesc') }}</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.account.authlibTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.account.authlibDesc') }}</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.account.offlineTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.account.offlineDesc') }}</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>

        <!-- 2. Mods -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-mods"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('mods')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div
                class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
              >
                <span class="i-lucide-package w-4 h-4" />
              </div>
              <div class="min-w-0">
                <h4 class="text-sm font-semibold text-fg">
                  {{ t('settings.about.questions.mods.title') }}
                </h4>
                <p class="text-xs text-fg-muted truncate mt-0.5">
                  {{ t('settings.about.questions.mods.desc') }}
                </p>
              </div>
            </div>
            <span
              class="i-lucide-chevron-down w-4 h-4 text-fg-muted transition-transform duration-200 shrink-0 ml-3"
              :class="{ 'rotate-180': openFaqs.mods }"
            />
          </button>
          <MCollapse :open="openFaqs.mods">
            <div
              data-testid="faq-content-mods"
              class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50 select-text"
            >
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.mods.loaderTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.mods.loaderDesc') }}</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.mods.marketTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.mods.marketDesc') }}</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.mods.manualTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.mods.manualDesc') }}</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>

        <!-- 3. Java -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-java"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('java')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div
                class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
              >
                <span class="i-lucide-coffee w-4 h-4" />
              </div>
              <div class="min-w-0">
                <h4 class="text-sm font-semibold text-fg">
                  {{ t('settings.about.questions.java.title') }}
                </h4>
                <p class="text-xs text-fg-muted truncate mt-0.5">
                  {{ t('settings.about.questions.java.desc') }}
                </p>
              </div>
            </div>
            <span
              class="i-lucide-chevron-down w-4 h-4 text-fg-muted transition-transform duration-200 shrink-0 ml-3"
              :class="{ 'rotate-180': openFaqs.java }"
            />
          </button>
          <MCollapse :open="openFaqs.java">
            <div
              data-testid="faq-content-java"
              class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50 select-text"
            >
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.java.versionTitle')
                  }}</span>
                  <ul class="list-disc list-inside mt-1 space-y-1 text-fg-muted">
                    <li>{{ t('settings.about.questions.java.v8') }}</li>
                    <li>{{ t('settings.about.questions.java.v16') }}</li>
                    <li>{{ t('settings.about.questions.java.v17') }}</li>
                    <li>{{ t('settings.about.questions.java.v21') }}</li>
                  </ul>
                </div>
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.java.autoTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.java.autoDesc') }}</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>

        <!-- 4. Crash -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-crash"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('crash')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div
                class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0"
              >
                <span class="i-lucide-triangle-alert w-4 h-4" />
              </div>
              <div class="min-w-0">
                <h4 class="text-sm font-semibold text-fg">
                  {{ t('settings.about.questions.crash.title') }}
                </h4>
                <p class="text-xs text-fg-muted truncate mt-0.5">
                  {{ t('settings.about.questions.crash.desc') }}
                </p>
              </div>
            </div>
            <span
              class="i-lucide-chevron-down w-4 h-4 text-fg-muted transition-transform duration-200 shrink-0 ml-3"
              :class="{ 'rotate-180': openFaqs.crash }"
            />
          </button>
          <MCollapse :open="openFaqs.crash">
            <div
              data-testid="faq-content-crash"
              class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50 select-text"
            >
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.crash.logTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.crash.logDesc') }}</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.crash.memTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.crash.memDesc') }}</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">{{
                    t('settings.about.questions.crash.bisectTitle')
                  }}</span>
                  <span>{{ t('settings.about.questions.crash.bisectDesc') }}</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>
      </div>
    </MSection>

    <!-- Tech Stack Section -->
    <MSection :title="t('settings.about.techStackTitle')" icon="i-lucide-cpu">
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs select-text">
        <div class="p-3 rounded-xl bg-sunken/40 border border-line flex items-center gap-3">
          <div
            class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0 font-bold"
          >
            🦀
          </div>
          <div>
            <span class="font-semibold text-fg">Rust & Tauri v2</span>
            <p class="text-fg-muted mt-0.5">{{ t('settings.about.techTauri') }}</p>
          </div>
        </div>
        <div class="p-3 rounded-xl bg-sunken/40 border border-line flex items-center gap-3">
          <div
            class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0 font-bold"
          >
            ⚡
          </div>
          <div>
            <span class="font-semibold text-fg">Vue 3 & UnoCSS</span>
            <p class="text-fg-muted mt-0.5">{{ t('settings.about.techVue') }}</p>
          </div>
        </div>
      </div>
    </MSection>
  </div>
</template>

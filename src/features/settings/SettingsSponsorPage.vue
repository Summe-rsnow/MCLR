<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { MCard, MImage, MSection } from '../../ui'

const { t } = useI18n()

/**
 * Payment QR codes are static files under `public/sponsor/` so they are served
 * from the app origin (`img-src 'self'` in the CSP) instead of being inlined.
 */
const channels = [
  {
    key: 'wechat',
    titleKey: 'settings.sponsor.wechat',
    icon: 'i-lucide-message-circle',
    src: '/sponsor/wechat.png',
  },
  {
    key: 'alipay',
    titleKey: 'settings.sponsor.alipay',
    icon: 'i-lucide-wallet',
    src: '/sponsor/alipay.png',
  },
]
</script>

<template>
  <div class="space-y-6 pb-12 select-none" data-testid="settings-sponsor-page">
    <MSection :title="t('settings.sponsor.title')" icon="i-lucide-heart">
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <MCard
          v-for="channel in channels"
          :key="channel.key"
          padding="lg"
          class="flex flex-col items-center gap-4"
        >
          <div class="flex items-center gap-2 text-sm font-semibold text-fg">
            <span :class="channel.icon" class="w-4 h-4 shrink-0 text-primary" />
            {{ t(channel.titleKey) }}
          </div>
          <div
            class="w-48 h-48 rounded-xl bg-raised border border-line overflow-hidden p-2"
            :data-testid="`sponsor-qr-${channel.key}`"
          >
            <MImage
              :src="channel.src"
              :alt="t(channel.titleKey)"
              fit="contain"
              loading="eager"
              class="w-full h-full"
            />
          </div>
          <p class="text-xs text-fg-muted text-center leading-relaxed">
            {{ t('settings.sponsor.scanHint') }}
          </p>
        </MCard>
      </div>
    </MSection>

    <MCard variant="tinted" padding="lg" class="space-y-1.5">
      <h3 class="text-sm font-semibold text-fg">{{ t('settings.sponsor.noteTitle') }}</h3>
      <p class="text-xs text-fg-muted leading-relaxed">{{ t('settings.sponsor.noteBody') }}</p>
    </MCard>
  </div>
</template>

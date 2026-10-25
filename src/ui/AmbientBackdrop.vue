<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useThemeStore } from '../stores/theme'

const themeStore = useThemeStore()
const isHidden = ref(typeof document !== 'undefined' ? document.hidden : false)
const isFocused = ref(typeof document !== 'undefined' ? (document.hasFocus?.() ?? true) : true)

function handleVisibilityChange() {
  if (typeof document !== 'undefined') {
    isHidden.value = document.hidden
  }
}
function onFocus() {
  isFocused.value = true
}
function onBlur() {
  isFocused.value = false
}

const paused = computed(() => isHidden.value || !isFocused.value || themeStore.motion !== 'full')

onMounted(() => {
  if (typeof document !== 'undefined') {
    document.addEventListener('visibilitychange', handleVisibilityChange)
  }
  if (typeof window !== 'undefined') {
    window.addEventListener('focus', onFocus)
    window.addEventListener('blur', onBlur)
  }
})

onUnmounted(() => {
  if (typeof document !== 'undefined') {
    document.removeEventListener('visibilitychange', handleVisibilityChange)
  }
  if (typeof window !== 'undefined') {
    window.removeEventListener('focus', onFocus)
    window.removeEventListener('blur', onBlur)
  }
})

const noiseDataUri =
  "data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23n)'/%3E%3C/svg%3E"
</script>

<template>
  <div
    v-if="themeStore.backgroundMode === 'none' && themeStore.material === 'glass'"
    data-testid="ambient-backdrop"
    class="pointer-events-none fixed inset-0 z-0 overflow-hidden"
  >
    <div class="ambient-blobs absolute inset-0" :class="{ 'ambient-paused': paused }">
      <div class="ambient-blob ambient-blob-1" />
      <div class="ambient-blob ambient-blob-2" />
      <div class="ambient-blob ambient-blob-3" />
    </div>
    <div
      class="absolute inset-0 opacity-[0.03]"
      :style="{ backgroundImage: `url(&quot;${noiseDataUri}&quot;)` }"
    />
  </div>
</template>

<style scoped>
.ambient-blobs {
  opacity: var(--ambient-strength, 0.55);
  filter: blur(60px);
}

.ambient-blob {
  position: absolute;
  border-radius: 9999px;
  will-change: transform;
  contain: strict;
  animation: ambient-drift 40s ease-in-out infinite alternate;
}

.ambient-paused .ambient-blob {
  animation-play-state: paused;
}

.ambient-blob-1 {
  width: 52vw;
  height: 52vh;
  left: -6%;
  top: -12%;
  background: radial-gradient(
    circle at 40% 40%,
    rgb(var(--ambient-1, 59 130 246) / 0.55),
    transparent 72%
  );
}

.ambient-blob-2 {
  width: 48vw;
  height: 48vh;
  right: -8%;
  top: -4%;
  background: radial-gradient(
    circle at 60% 40%,
    rgb(var(--ambient-2, 99 102 241) / 0.48),
    transparent 72%
  );
  animation-delay: -13s;
}

.ambient-blob-3 {
  width: 55vw;
  height: 50vh;
  left: 32%;
  bottom: -18%;
  background: radial-gradient(
    circle at 50% 60%,
    rgb(var(--ambient-3, 20 184 166) / 0.45),
    transparent 72%
  );
  animation-delay: -27s;
}

@keyframes ambient-drift {
  0% {
    transform: translate3d(-3%, -2%, 0) scale(1);
  }
  100% {
    transform: translate3d(3%, 3%, 0) scale(1.05);
  }
}
</style>

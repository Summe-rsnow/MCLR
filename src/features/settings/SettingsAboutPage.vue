<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { openUrl } from '@tauri-apps/plugin-opener'
import {
  MCard,
  MSection,
  MButton,
  MBadge,
  MCollapse,
} from '../../ui'

const { t } = useI18n()

const appVersion = 'v0.0.16'
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
          <div class="w-14 h-14 rounded-2xl bg-primary flex items-center justify-center text-primary-on shadow-2 shrink-0">
            <span class="i-lucide-rocket w-8 h-8" />
          </div>
          <div class="space-y-1">
            <div class="flex items-center gap-2.5">
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
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-github"
            @click="openExternal(repoUrl)"
          >
            {{ t('settings.about.githubRepo') }}
          </MButton>
          <MButton
            variant="tonal"
            size="sm"
            icon="i-lucide-message-square-plus"
            @click="openExternal(issuesUrl)"
          >
            {{ t('settings.about.feedbackIssues') }}
          </MButton>
        </div>
      </div>
    </MCard>

    <!-- FAQ & Help Guide Section -->
    <MSection
      :title="t('settings.about.faqTitle')"
      :description="t('settings.about.faqSubtitle')"
      icon="i-lucide-help-circle"
    >
      <div class="space-y-3">
        <!-- 1. 账号添加 -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-account"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('account')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
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
            <div data-testid="faq-content-account" class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50">
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">微软正版账号：</span>
                  <span>推荐拥有 Minecraft 正版的玩家使用。点击“账号管理” -> “添加账号” -> “微软登录”，通过浏览器打开微软授权页并输入设备验证码即可绑定，自动同步正版皮肤。</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">第三方登录 (authlib-injector)：</span>
                  <span>支持 LittleSkin、统一通行证等第三方认证服务。输入认证服务器 URL 或拖拽外置登录配置链接，输入邮箱与密码即可登录。</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">离线模式账号：</span>
                  <span>无需联网验证，可直接输入自定义游戏 ID。适用于局域网联机或离线体验。</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>

        <!-- 2. 模组安装 -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-mods"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('mods')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
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
            <div data-testid="faq-content-mods" class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50">
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">模组加载器安装：</span>
                  <span>进入“下载中心”，选择游戏版本时直接勾选 Fabric、Forge、NeoForge 或 Quilt，启动器会自动完成对应加载器的文件补丁与依赖下载。</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">组件市场一键安装：</span>
                  <span>点击左侧“社区资源”或实例详情页中的“模组”，即可直接在线搜索 CurseForge 与 Modrinth 的数十万款模组，支持依赖自动解析与版本匹配。</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">手动安装模组：</span>
                  <span>进入实例管理页点击“打开游戏文件夹”，将下载好的 <code>.jar</code> 模组文件直接拖入 <code>mods</code> 文件夹中即可。</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>

        <!-- 3. Java 版本选择 -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-java"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('java')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
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
            <div data-testid="faq-content-java" class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50">
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">版本对应关系推荐：</span>
                  <ul class="list-disc list-inside mt-1 space-y-1 text-fg-muted">
                    <li><strong class="text-fg">1.16.5 及更早版本</strong>：需要 Java 8 (JRE 8)</li>
                    <li><strong class="text-fg">1.17 - 1.17.1</strong>：需要 Java 16</li>
                    <li><strong class="text-fg">1.18 - 1.20.4</strong>：需要 Java 17</li>
                    <li><strong class="text-fg">1.20.5 及最新版本</strong>：需要 Java 21</li>
                  </ul>
                </div>
                <div>
                  <span class="font-semibold text-fg">自动下载与匹配：</span>
                  <span>启动器会在启动时自动检测并优先匹配最契合的 Java。如果本机缺失合适运行时，点击启动页面的“自动下载 Java”按钮即可秒级下载 Mojang 官方运行时。</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>

        <!-- 4. 游戏闪退排查 -->
        <MCard padding="none" class="overflow-hidden transition-all">
          <button
            data-testid="faq-btn-crash"
            class="w-full p-4 flex items-center justify-between text-left hover:bg-sunken/50 transition-colors"
            @click="toggleFaq('crash')"
          >
            <div class="flex items-center gap-3 min-w-0">
              <div class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0">
                <span class="i-lucide-alert-triangle w-4 h-4" />
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
            <div data-testid="faq-content-crash" class="px-4 pb-4 pt-1 text-xs text-fg-muted leading-relaxed space-y-2 border-t border-line/50">
              <div class="p-3 rounded-lg bg-sunken/40 space-y-2">
                <div>
                  <span class="font-semibold text-fg">查看日志与崩溃分析：</span>
                  <span>发生崩溃时启动器会自动弹出崩溃分析对话框，精准提示崩溃原因（如缺少前置模组、内存溢出、显卡驱动不兼容等）。也可点击“查看日志”寻找带有 <code>FATAL</code> 或 <code>Exception</code> 的错误行。</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">内存设置：</span>
                  <span>进入“设置” -> “启动设置”，确保为游戏分配了合理内存（通常推荐 4096MB - 8192MB）。</span>
                </div>
                <div>
                  <span class="font-semibold text-fg">模组排查（二分法）：</span>
                  <span>若崩溃由模组引起，可先将 <code>mods</code> 文件夹中的一半模组移出测试，逐步缩小范围定位冲突模组。</span>
                </div>
              </div>
            </div>
          </MCollapse>
        </MCard>
      </div>
    </MSection>

    <!-- Tech Stack Section -->
    <MSection
      :title="t('settings.about.techStackTitle')"
      icon="i-lucide-cpu"
    >
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
        <div class="p-3 rounded-xl bg-sunken/40 border border-line flex items-center gap-3">
          <div class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0 font-bold">
            🦀
          </div>
          <div>
            <span class="font-semibold text-fg">Rust & Tauri v2</span>
            <p class="text-fg-muted mt-0.5">{{ t('settings.about.techTauri') }}</p>
          </div>
        </div>
        <div class="p-3 rounded-xl bg-sunken/40 border border-line flex items-center gap-3">
          <div class="w-8 h-8 rounded-lg bg-primary-soft text-primary-soft-fg flex items-center justify-center shrink-0 font-bold">
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

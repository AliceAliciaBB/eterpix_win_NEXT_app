<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAppStore } from './stores/appStore'
import LoginPage from './components/LoginPage.vue'
import HomePage from './components/HomePage.vue'
import SettingsPage from './components/SettingsPage.vue'

const store = useAppStore()

// テーマ管理
const theme = ref<'light' | 'dark'>('light')

function initTheme() {
  const saved = localStorage.getItem('theme')
  if (saved === 'dark' || saved === 'light') {
    theme.value = saved
  } else if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
    theme.value = 'dark'
  }
  document.documentElement.setAttribute('data-theme', theme.value)
}

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
  localStorage.setItem('theme', theme.value)
  document.documentElement.setAttribute('data-theme', theme.value)
}

async function hideToTray() {
  await getCurrentWindow().hide()
}

onMounted(async () => {
  initTheme()
  try {
    await store.setupEventListeners()
  } catch (e) {
    console.error('setupEventListeners error', e)
  }
  await store.fetchStatus()

  // タイトルバーの最小化ボタン → トレイへ格納
  try {
    const win = getCurrentWindow()
    await win.onResized(async () => {
      if (await win.isMinimized()) {
        await win.hide()
      }
    })
  } catch (e) {
    console.error('window setup error', e)
  }
})
</script>

<template>
  <div class="app-shell">
    <!-- 初期化中ローディング -->
    <div v-if="store.isInitializing" class="loading-screen">
      <div class="loading-spinner"></div>
    </div>

    <!-- ログインページ -->
    <LoginPage v-else-if="store.page === 'login'" />

    <!-- メインコンテンツ -->
    <template v-else>
      <div class="main-layout">
      <div class="container">
        <!-- ヘッダー -->
        <header class="header">
          <img src="/icons/icon.png" alt="" class="logo" />
          <div>
            <h1>EterPix VRC Uploader</h1>
            <p class="subtitle">{{ store.status?.last_status ?? '接続中...' }}</p>
          </div>
          <span class="header-spacer" />
          <button class="theme-toggle" @click="toggleTheme" title="テーマ切替">
            {{ theme === 'dark' ? '☀' : '☽' }}
          </button>
        </header>

        <!-- ページ本体 -->
        <HomePage v-if="store.page === 'home'" />
        <SettingsPage v-else-if="store.page === 'settings'" />
      </div>

      <!-- 下部ナビゲーション -->
      <nav class="bottom-nav">
        <button
          class="bottom-nav-item"
          :class="{ active: store.page === 'home' }"
          @click="store.page = 'home'"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
            <polyline points="9 22 9 12 15 12 15 22"/>
          </svg>
          <span>ホーム</span>
        </button>
        <button
          class="bottom-nav-item"
          :class="{ active: store.page === 'settings' }"
          @click="store.page = 'settings'"
        >
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="3"/>
            <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
          </svg>
          <span>設定</span>
        </button>
      </nav>
      </div>
    </template>

    <!-- トースト通知 -->
    <div class="toast" :class="{ show: !!store.toast }">
      {{ store.toast }}
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  min-height: 100vh;
  position: relative;
}
</style>

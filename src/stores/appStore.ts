// appStore.ts - Pinia ストア (アプリ全状態 + Tauriコマンド呼び出し)

import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { ref, computed } from 'vue'

export interface UploadHistoryItem {
  filename: string
  photo_uuid: string | null
  time: string
}

export interface AppStatus {
  logged_in: boolean
  username: string | null
  server_url: string
  watch_folder: string
  default_visibility: string
  visibility_options: [string, string][]
  is_watching: boolean
  is_offline: boolean
  queue_count: number
  osc_running: boolean
  osc_recv: number | null
  osc_current: string | null
  world: { world_id: string; instance_id: string } | null
  upload_history: UploadHistoryItem[]
  last_status: string
  startup_registered: boolean
  auto_upload: boolean
  jpeg_quality: number
  osc_send_port: number
  osc_recv_port: number
}

export interface ApiResponse {
  status: string
  message?: string
  data?: Record<string, unknown>
}

export const useAppStore = defineStore('app', () => {
  // ========== 状態 ==========
  const status = ref<AppStatus | null>(null)
  const toast = ref<string>('')
  const toastTimer = ref<ReturnType<typeof setTimeout> | null>(null)
  const page = ref<'home' | 'settings' | 'login'>('login')
  const isLoading = ref(false)
  const isInitializing = ref(true)

  // ========== Computed ==========
  const isLoggedIn = computed(() => status.value?.logged_in ?? false)

  // ========== トースト通知 ==========
  function showToast(msg: string) {
    toast.value = msg
    if (toastTimer.value) clearTimeout(toastTimer.value)
    toastTimer.value = setTimeout(() => { toast.value = '' }, 3000)
  }

  // ========== 全状態を取得 ==========
  async function fetchStatus() {
    try {
      const s = await invoke<AppStatus>('get_status')
      status.value = s
      if (s.logged_in) {
        // ログイン済み → ログイン画面にいたらホームへ
        if (page.value === 'login') page.value = 'home'
      } else {
        page.value = 'login'
      }
    } catch (e) {
      console.error('get_status error', e)
      page.value = 'login'
    } finally {
      isInitializing.value = false
    }
  }

  // ========== ログイン ==========
  async function login(username: string, password: string): Promise<ApiResponse> {
    const resp = await invoke<ApiResponse>('login', { username, password })
    if (resp.status === 'success') {
      await fetchStatus()
      page.value = 'home'
    }
    return resp
  }

  // ========== 新規登録 ==========
  async function register(username: string, password: string): Promise<ApiResponse> {
    const resp = await invoke<ApiResponse>('register_user', { username, password })
    if (resp.status === 'success') {
      await fetchStatus()
      page.value = 'home'
    }
    return resp
  }

  // ========== ログアウト ==========
  async function logout() {
    await invoke('logout')
    await fetchStatus()
    page.value = 'login'
  }

  // ========== 監視トグル ==========
  async function toggleWatch() {
    const isWatching = await invoke<boolean>('toggle_watch')
    showToast(isWatching ? '監視を開始しました' : '監視を停止しました')
    await fetchStatus()
  }

  // ========== OSC トグル ==========
  async function toggleOsc() {
    const isRunning = await invoke<boolean>('toggle_osc')
    showToast(isRunning ? 'OSC を開始しました' : 'OSC を停止しました')
    await fetchStatus()
  }

  // ========== サーバー確認 ==========
  async function checkServer(): Promise<boolean> {
    const alive = await invoke<boolean>('check_server')
    showToast(alive ? 'サーバー: オンライン' : 'サーバー: オフライン')
    await fetchStatus()
    return alive
  }

  // ========== キュー再送 ==========
  async function resendQueue(): Promise<boolean> {
    const ok = await invoke<boolean>('resend_queue')
    showToast(ok ? '再送信完了' : 'サーバーに接続できません')
    await fetchStatus()
    return ok
  }

  // ========== 設定保存 ==========
  async function saveSettings(settings: {
    server_url?: string
    watch_folder?: string
    default_visibility?: string
    auto_upload?: boolean
    jpeg_quality?: number
    osc_send_port?: number
    osc_recv_port?: number
  }) {
    await invoke('save_settings', settings)
    await fetchStatus()
  }

  // ========== スタートアップ切替 ==========
  async function toggleStartup(): Promise<boolean> {
    const registered = await invoke<boolean>('toggle_startup')
    showToast(registered ? 'スタートアップに登録しました' : 'スタートアップから解除しました')
    await fetchStatus()
    return registered
  }

  // ========== リアルタイムイベント受信 ==========
  async function setupEventListeners() {
    await listen<{ message: string }>('status', ({ payload }) => {
      if (status.value) status.value.last_status = payload.message
      showToast(payload.message)
    })

    await listen('upload_start', () => {
      showToast('アップロード中...')
    })

    await listen<{ photo_uuid?: string }>('upload_complete', async () => {
      showToast('アップロード完了')
      await fetchStatus()
    })

    await listen<{ error: string }>('upload_error', ({ payload }) => {
      showToast('エラー: ' + payload.error)
    })

    await listen<{ world_id: string; instance_id: string }>('world_joined', ({ payload }) => {
      if (status.value) {
        status.value.world = { world_id: payload.world_id, instance_id: payload.instance_id }
      }
    })

    await listen('world_left', async () => {
      if (status.value) status.value.world = null
    })

    await listen<{ is_offline: boolean }>('offline_mode', ({ payload }) => {
      if (status.value) status.value.is_offline = payload.is_offline
      fetchStatus()
    })

    await listen('photo_queued', () => { fetchStatus() })
    await listen('queue_item_sent', () => { fetchStatus() })
    await listen('queue_processed', () => { fetchStatus() })

    await listen<{ visibility: string }>('osc_visibility_changed', ({ payload }) => {
      showToast('OSC: 公開範囲を ' + payload.visibility + ' に変更')
      if (status.value) {
        status.value.default_visibility = payload.visibility
        status.value.osc_current = payload.visibility
      }
    })

    await listen('osc_started', () => { fetchStatus() })
    await listen('osc_stopped', () => { fetchStatus() })
  }

  return {
    status,
    toast,
    page,
    isLoading,
    isInitializing,
    isLoggedIn,
    showToast,
    fetchStatus,
    login,
    register,
    logout,
    toggleWatch,
    toggleOsc,
    checkServer,
    resendQueue,
    saveSettings,
    toggleStartup,
    setupEventListeners,
  }
})

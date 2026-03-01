<script setup lang="ts">
import { ref, watch } from 'vue'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()

// ローカル編集用フォーム
const serverUrl = ref('')
const watchFolder = ref('')

watch(() => store.status, (s) => {
  if (s) {
    serverUrl.value = s.server_url
    watchFolder.value = s.watch_folder
  }
}, { immediate: true })

async function saveGeneral() {
  await store.saveSettings({
    server_url: serverUrl.value,
    watch_folder: watchFolder.value,
  })
}
</script>

<template>
  <div v-if="store.status">
    <!-- 一般設定 -->
    <section class="card">
      <h2>設定</h2>

      <div class="field">
        <label for="serverUrl">サーバーURL</label>
        <input id="serverUrl" v-model="serverUrl" type="text" placeholder="https://..." />
      </div>

      <div class="field">
        <label for="watchFolder">監視フォルダ</label>
        <input
          id="watchFolder"
          v-model="watchFolder"
          type="text"
          placeholder="C:\Users\...\Pictures\VRChat"
        />
      </div>

      <button class="btn btn-small" @click="saveGeneral">設定を保存</button>
    </section>

    <!-- OSC -->
    <section class="card">
      <h2>OSC</h2>
      <div class="status-item">
        <span class="status-value">
          <span class="dot" :class="store.status.osc_running ? 'dot-green' : 'dot-gray'"></span>
          {{ store.status.osc_running ? '動作中' : '停止中' }}
        </span>
        <button class="btn btn-small" @click="store.toggleOsc()">
          {{ store.status.osc_running ? '停止' : '開始' }}
        </button>
      </div>
      <p v-if="store.status.osc_running" class="hint" style="margin-top:8px">
        受信値: {{ store.status.osc_recv ?? '-' }} /
        現在の公開範囲: {{ store.status.osc_current ?? '-' }}
      </p>
      <div class="divider"></div>
      <div class="field">
        <label>送信ポート (VRC向け)</label>
        <input
          type="number"
          :value="store.status.osc_send_port"
          @change="store.saveSettings({ osc_send_port: +($event.target as HTMLInputElement).value })"
          min="1024"
          max="65535"
        />
      </div>
      <div class="field">
        <label>受信ポート (VRCから)</label>
        <input
          type="number"
          :value="store.status.osc_recv_port"
          @change="store.saveSettings({ osc_recv_port: +($event.target as HTMLInputElement).value })"
          min="1024"
          max="65535"
        />
      </div>
    </section>

    <!-- アップロード詳細 -->
    <section class="card">
      <h2>アップロード</h2>
      <div class="field">
        <label>JPEG品質 ({{ store.status.jpeg_quality }})</label>
        <input
          type="range"
          :value="store.status.jpeg_quality"
          min="60"
          max="100"
          @change="store.saveSettings({ jpeg_quality: +($event.target as HTMLInputElement).value })"
        />
      </div>
      <div class="status-item" style="margin-top:8px">
        <span class="status-label">自動アップロード</span>
        <label class="toggle-switch">
          <input
            type="checkbox"
            :checked="store.status.auto_upload"
            @change="store.saveSettings({ auto_upload: ($event.target as HTMLInputElement).checked })"
          />
          <span class="toggle-thumb"></span>
        </label>
      </div>
    </section>

    <!-- スタートアップ -->
    <section class="card">
      <h2>Windows スタートアップ</h2>
      <div class="status-item">
        <span class="status-value">
          <span class="dot" :class="store.status.startup_registered ? 'dot-green' : 'dot-gray'"></span>
          {{ store.status.startup_registered ? '登録済み' : '未登録' }}
        </span>
        <button class="btn btn-small" @click="store.toggleStartup()">
          {{ store.status.startup_registered ? '解除' : '登録' }}
        </button>
      </div>
      <p class="hint">登録するとWindowsログオン時に自動起動します（トレイに最小化）</p>
    </section>
  </div>
</template>

<style scoped>
.toggle-switch {
  position: relative;
  display: inline-block;
  width: 44px;
  height: 24px;
  cursor: pointer;
}
.toggle-switch input {
  opacity: 0;
  width: 0;
  height: 0;
}
.toggle-thumb {
  position: absolute;
  inset: 0;
  background: var(--border);
  border-radius: 12px;
  transition: background 0.2s;
}
.toggle-thumb::before {
  content: '';
  position: absolute;
  left: 3px;
  top: 3px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s;
}
input:checked + .toggle-thumb {
  background: var(--accent);
}
input:checked + .toggle-thumb::before {
  transform: translateX(20px);
}
input[type="range"] {
  width: 100%;
  accent-color: var(--accent);
  padding: 0;
  border: none;
  background: none;
}
</style>

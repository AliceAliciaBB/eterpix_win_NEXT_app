<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()
const serverChecking = ref(false)
const resending = ref(false)

async function handleCheckServer() {
  serverChecking.value = true
  await store.checkServer()
  serverChecking.value = false
}

async function handleResend() {
  resending.value = true
  await store.resendQueue()
  resending.value = false
}

async function handleVisibilityChange(e: Event) {
  const val = (e.target as HTMLSelectElement).value
  await store.saveSettings({ default_visibility: val })
}
</script>

<template>
  <div v-if="store.status">
    <!-- アカウントバー -->
    <section class="card" id="accountSection">
      <div class="account-bar">
        <span class="dot dot-green"></span>
        <span class="account-name">{{ store.status.username ?? 'ユーザー' }}</span>
        <span class="header-spacer"></span>
        <button class="btn btn-small btn-muted" @click="store.logout()">ログアウト</button>
      </div>
    </section>

    <!-- 監視状態バナー -->
    <section class="card watch-card" :class="{ 'is-watching': store.status.is_watching }">
      <div class="watch-banner">
        <span class="watch-indicator"></span>
        <span class="watch-label">{{ store.status.is_watching ? '監視中' : '停止中' }}</span>
      </div>
      <button
        class="btn watch-toggle"
        :class="store.status.is_watching ? 'btn-muted' : 'btn-primary'"
        @click="store.toggleWatch()"
      >
        {{ store.status.is_watching ? '監視を停止' : '監視を開始' }}
      </button>
    </section>

    <!-- 公開範囲 -->
    <section class="card">
      <div class="field" style="margin-bottom:0">
        <label for="visibility">公開範囲</label>
        <select id="visibility" :value="store.status.default_visibility" @change="handleVisibilityChange">
          <option
            v-for="[key, label] in store.status.visibility_options"
            :key="key"
            :value="key"
          >{{ label }}</option>
        </select>
      </div>
    </section>

    <!-- 状態 -->
    <section class="card">
      <h2>状態</h2>
      <div class="status-grid">
        <!-- サーバー -->
        <div class="status-item">
          <span class="status-label">サーバー</span>
          <span class="status-value">
            <template v-if="store.status.is_offline">
              <span class="dot dot-red"></span> オフライン
            </template>
            <template v-else>
              <span class="dot dot-gray"></span> 未確認
            </template>
          </span>
          <button class="btn btn-small" :disabled="serverChecking" @click="handleCheckServer">
            {{ serverChecking ? '...' : '確認' }}
          </button>
        </div>

        <!-- 送信待ち -->
        <div class="status-item">
          <span class="status-label">送信待ち</span>
          <span class="status-value">
            <template v-if="store.status.queue_count > 0">
              <span class="dot dot-orange"></span>
              {{ store.status.queue_count }}件
            </template>
            <template v-else>0件</template>
          </span>
          <button
            class="btn btn-small"
            :disabled="store.status.queue_count === 0 || resending"
            @click="handleResend"
          >
            {{ resending ? '...' : '再送信' }}
          </button>
        </div>

        <!-- ワールド -->
        <div class="status-item">
          <span class="status-label">ワールド</span>
          <span class="status-value" style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap">
            <template v-if="store.status.world">
              {{ store.status.world.world_id }}:{{ store.status.world.instance_id }}
            </template>
            <template v-else>-</template>
          </span>
        </div>
      </div>
    </section>

    <!-- アップロード履歴 -->
    <section class="card">
      <h2>アップロード履歴</h2>
      <div v-if="store.status.upload_history.length === 0">
        <p class="hint">まだアップロードはありません</p>
      </div>
      <div v-else>
        <div
          v-for="item in store.status.upload_history"
          :key="item.filename + item.time"
          class="history-item"
        >
          <span class="history-time">{{ item.time }}</span>
          <span class="history-name">{{ item.filename }}</span>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useAppStore } from '../stores/appStore'

const store = useAppStore()

const mode = ref<'login' | 'register'>('login')
const username = ref('')
const password = ref('')
const errorMsg = ref('')
const loading = ref(false)

async function submit() {
  errorMsg.value = ''
  if (!username.value || !password.value) {
    errorMsg.value = 'ユーザー名とパスワードを入力してください'
    return
  }
  if (mode.value === 'register') {
    if (username.value.length < 3 || username.value.length > 20) {
      errorMsg.value = 'ユーザー名は3〜20文字で入力してください'
      return
    }
    if (password.value.length < 6) {
      errorMsg.value = 'パスワードは6文字以上で入力してください'
      return
    }
  }

  loading.value = true
  try {
    const resp = mode.value === 'login'
      ? await store.login(username.value, password.value)
      : await store.register(username.value, password.value)

    if (resp.status !== 'success') {
      errorMsg.value = resp.message ?? 'エラーが発生しました'
    }
  } catch (e) {
    errorMsg.value = String(e)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="login-shell">
    <div class="login-card">
      <img src="/icons/icon.png" alt="EterPix" class="login-logo" />
      <h1 class="login-title">EterPix VRC Uploader</h1>
      <p class="login-subtitle">{{ mode === 'login' ? 'ログイン' : '新規登録' }}</p>

      <form @submit.prevent="submit" class="login-form">
        <div class="field">
          <label for="username">ユーザー名</label>
          <input
            id="username"
            v-model="username"
            type="text"
            placeholder="username"
            autocomplete="username"
          />
        </div>
        <div class="field">
          <label for="password">パスワード</label>
          <input
            id="password"
            v-model="password"
            type="password"
            placeholder="••••••"
            autocomplete="current-password"
          />
        </div>

        <p v-if="errorMsg" class="error-msg">{{ errorMsg }}</p>

        <button type="submit" class="btn btn-primary submit-btn" :disabled="loading">
          {{ loading ? '処理中...' : (mode === 'login' ? 'ログイン' : '登録') }}
        </button>
      </form>

      <button class="toggle-mode" @click="mode = mode === 'login' ? 'register' : 'login'">
        {{ mode === 'login' ? 'アカウントを新規作成' : 'ログインに戻る' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.login-shell {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  padding: 24px;
  background: var(--bg);
}
.login-card {
  width: 100%;
  max-width: 360px;
  background: var(--card);
  border-radius: 16px;
  padding: 32px 24px;
  box-shadow: 0 4px 24px rgba(0,0,0,0.10);
  text-align: center;
}
.login-logo {
  width: 64px;
  height: 64px;
  border-radius: 16px;
  margin-bottom: 16px;
}
.login-title {
  font-size: 20px;
  font-weight: 700;
  margin-bottom: 4px;
}
.login-subtitle {
  font-size: 14px;
  color: var(--text-secondary);
  margin-bottom: 24px;
}
.login-form {
  text-align: left;
}
.submit-btn {
  width: 100%;
  margin-top: 8px;
  padding: 12px;
  font-size: 15px;
}
.error-msg {
  font-size: 13px;
  color: var(--red);
  margin-top: 4px;
  margin-bottom: 4px;
}
.toggle-mode {
  display: block;
  margin-top: 20px;
  font-size: 13px;
  color: var(--accent);
  background: none;
  border: none;
  cursor: pointer;
  width: 100%;
}
.toggle-mode:hover {
  text-decoration: underline;
}
</style>

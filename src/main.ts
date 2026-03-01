import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './style.css'

// Ctrl+R / F5 / Ctrl+Shift+R によるリロードを無効化
window.addEventListener('keydown', (e) => {
  if (
    e.key === 'F5' ||
    (e.ctrlKey && (e.key === 'r' || e.key === 'R'))
  ) {
    e.preventDefault()
  }
}, { capture: true })

const app = createApp(App)
app.use(createPinia())
app.mount('#app')

/**
 * 根组件：全局一次性的 ToastHost（useToast 通知）与 ProgressBar（useProgress 顶部进度条）。
 * 视图通过 <router-view /> 渲染。
 * 另挂全局 error/unhandledrejection 监听：把未捕获的异常通过 console + toast 暴露，
 * 便于在没有 DevTools 的情况下定位前端问题。
 * 监听 macOS 原生菜单「About RustFox」事件以打开自定义关于弹窗。
 */
<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import ToastHost from './components/ToastHost.vue'
import ProgressBar from './components/ProgressBar.vue'
import Brand from './components/Brand.vue'
import AboutDialog from './components/AboutDialog.vue'
import { useToast } from './composables/useToast'

const toast = useToast()
const route = useRoute()

/** 仪表板 / 工作区页自带顶栏品牌（工作区顶部栏已内嵌品牌），隐藏全局浮层品牌避免重复。 */
const showFloatingBrand = computed(
  () => route.path !== '/projects' && route.path !== '/workspace',
)

const showAbout = ref(false)
let unlistenAbout: UnlistenFn | null = null

onMounted(async () => {
  window.addEventListener('error', (event) => {
    console.error('[window.error]', event.message, event.error)
    const msg = String(event.error?.message ?? event.message)
    toast.error('页面错误', { message: msg, duration: 6000 })
  })
  window.addEventListener('unhandledrejection', (event) => {
    const reason = event.reason
    console.error('[unhandledrejection]', reason)
    const msg = reason instanceof Error ? reason.message : String(reason)
    toast.error('未处理的 Promise 错误', { message: msg, duration: 6000 })
  })
  try {
    if ('__TAURI_INTERNALS__' in window) {
      unlistenAbout = await listen('rustfox://about', () => {
        showAbout.value = true
      })
    }
  } catch {
    // 非 Tauri（浏览器预览）环境：忽略
  }
})

onBeforeUnmount(() => {
  unlistenAbout?.()
})
</script>

<template>
  <div v-if="showFloatingBrand" class="app-brand" aria-label="RustFox 品牌">
    <Brand title="RustFox" subtitle="API 调试工具" />
  </div>
  <ProgressBar />
  <ToastHost />
  <AboutDialog v-model:open="showAbout" />
  <main class="rf-app">
    <router-view />
  </main>
</template>

<style scoped>
.app-brand {
  position: fixed;
  top: 8px;
  left: 8px;
  z-index: 60;
  padding: 2px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow);
}
.app-brand :deep(.brand) {
  min-width: 0;
  width: 132px;
}
</style>
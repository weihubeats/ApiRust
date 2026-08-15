/**
 * 根组件：全局一次性的 ToastHost（useToast 通知）与 ProgressBar（useProgress 顶部进度条）。
 * 视图通过 <router-view /> 渲染。
 * 另挂全局 error/unhandledrejection 监听：把未捕获的异常通过 console + toast 暴露，
 * 便于在没有 DevTools 的情况下定位前端问题。
 */
<script setup lang="ts">
import { onMounted } from 'vue'
import ToastHost from './components/ToastHost.vue'
import ProgressBar from './components/ProgressBar.vue'
import { useToast } from './composables/useToast'

const toast = useToast()

onMounted(() => {
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
})
</script>

<template>
  <ProgressBar />
  <ToastHost />
  <main class="rf-app">
    <router-view />
  </main>
</template>
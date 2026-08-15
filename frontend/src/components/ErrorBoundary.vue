<script setup lang="ts">
/**
 * ErrorBoundary：Vue 错误边界组件。
 *
 * Vue 没有 React 式内置错误边界，这里用 `onErrorCaptured` 实现：
 * 捕获子树中所有渲染 / 事件处理 / 生命周期抛出的未处理错误，
 * 渲染 rf- 风格兜底 UI，并提供「重试」按钮。
 *
 * 使用：
 * ```html
 * <ErrorBoundary :retry="loadProjects" @error="onError">
 *   <ProjectList />
 * </ErrorBoundary>
 * ```
 * - `retry`：重试回调（重新加载数据后返回 Promise）；不传则只重置边界。
 * - 边界捕获错误后子内容不渲染（防止无限崩溃循环）；
 *   `onError` 事件可接 useToast 弹通知。
 *
 * 全局兜底（错误边界外的错误）：在 main.ts 注册
 * ```ts
 * app.config.errorHandler = (err, _instance, info) => {
 *   console.error('[global]', info, err)
 * }
 * ```
 * 两条路径是互补的：边界保 UI 可恢复，全局 handler 保证任何错误都有日志出口。
 */
import { onErrorCaptured, ref } from 'vue'
import { useToast } from '../composables/useToast'

const props = withDefaults(
  defineProps<{
    /** 重试回调（重新执行加载逻辑）；不传则只清除错误状态重新渲染。 */
    retry?: () => Promise<unknown> | void
  }>(),
  {},
)

const emit = defineEmits<{
  (e: 'error', err: Error, info: string): void
}>()

const toast = useToast()
const errored = ref(false)
const errorMessage = ref('')
const errorStack = ref('')
const errorInfo = ref('')
const detailOpen = ref(false)
const retrying = ref(false)

onErrorCaptured((err, _instance, info) => {
  errored.value = true
  const e = err instanceof Error ? err : new Error(String(err))
  errorMessage.value = e.message || String(err)
  errorStack.value = e.stack ?? ''
  errorInfo.value = info
  emit('error', e, info)
  // 返回 false：阻止错误继续向上冒泡（避免全局 handler 重复上报）。
  return false
})

async function onRetry(): Promise<void> {
  retrying.value = true
  try {
    if (props.retry) {
      await props.retry()
    }
    errored.value = false
    errorMessage.value = ''
    errorStack.value = ''
    errorInfo.value = ''
    detailOpen.value = false
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : String(e)
    toast.error('重试失败', { message: errorMessage.value })
  } finally {
    retrying.value = false
  }
}
</script>

<template>
  <div v-if="errored" class="rf-boundary" role="alert">
    <div class="rf-boundary-glyph" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="28" height="28">
        <path
          d="M12 3l9.5 16.5H2.5L12 3z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linejoin="round"
        />
        <path
          d="M12 9.5v4.5"
          stroke="currentColor"
          stroke-width="1.6"
          stroke-linecap="round"
        />
        <circle cx="12" cy="16.6" r="0.9" fill="currentColor" />
      </svg>
    </div>
    <div class="rf-boundary-body">
      <div class="rf-boundary-title">页面出现异常</div>
      <div class="rf-boundary-message">{{ errorMessage || '未知错误' }}</div>
      <div class="rf-boundary-actions">
        <button
          class="rf-btn rf-btn-primary rf-btn-sm"
          type="button"
          :disabled="retrying"
          @click="onRetry"
        >
          {{ retrying ? '重试中…' : '重试' }}
        </button>
        <button
          v-if="errorStack || errorInfo"
          class="rf-btn rf-btn-sm"
          type="button"
          @click="detailOpen = !detailOpen"
        >
          {{ detailOpen ? '收起详情' : '查看详情' }}
        </button>
      </div>
      <pre v-if="detailOpen" class="rf-boundary-detail">{{
        errorInfo ? `[${errorInfo}]\n` : ''
      }}{{ errorStack || errorMessage }}</pre>
    </div>
  </div>
  <slot v-else />
</template>

<style scoped>
.rf-boundary {
  display: flex;
  gap: 14px;
  margin: 24px;
  padding: 20px;
  border-radius: 10px;
  background: var(--rf-bg-panel-2);
  border: 1px solid var(--rf-border);
  border-left: 3px solid var(--rf-danger);
}

.rf-boundary-glyph {
  flex: none;
  color: var(--rf-danger);
  margin-top: 2px;
}

.rf-boundary-body {
  flex: 1;
  min-width: 0;
}

.rf-boundary-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--rf-text);
}

.rf-boundary-message {
  margin-top: 4px;
  font-size: 12.5px;
  color: var(--rf-text-secondary);
  word-break: break-all;
}

.rf-boundary-actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}

.rf-boundary-detail {
  margin-top: 12px;
  padding: 10px;
  border-radius: 6px;
  background: var(--rf-input-bg);
  border: 1px solid var(--rf-border);
  font-size: 11.5px;
  line-height: 1.5;
  color: var(--rf-text-secondary);
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 220px;
  overflow: auto;
}
</style>
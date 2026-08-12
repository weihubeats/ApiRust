<script setup lang="ts">
/**
 * ToastHost：渲染全局 Toast（配合 composables/useToast.ts）。
 *
 * 在 App 根组件挂载一次即可：
 * ```html
 * <ToastHost />
 * ```
 * 样式为 rf- 设计系统的 Vue 侧镜像（scoped + 变量对齐 styles.rs：
 * --rf-bg-panel / --rf-border / --rf-text / 语义色）。
 */
import { TOAST_TYPE_META, useToast } from '../composables/useToast'
import type { ToastItem, ToastType } from '../composables/useToast'

const { toasts, dismiss } = useToast()

const ICONS: Record<ToastType, string> = {
  success:
    '<svg viewBox="0 0 16 16" width="16" height="16"><path d="M3 8.5l3.2 3.2L13 5" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>',
  info: '<svg viewBox="0 0 16 16" width="16" height="16"><circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M8 7.2v4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="8" cy="4.8" r="1" fill="currentColor"/></svg>',
  warning:
    '<svg viewBox="0 0 16 16" width="16" height="16"><path d="M8 2L1.5 13.5h13L8 2z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/><path d="M8 6.4v3.4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/><circle cx="8" cy="11.5" r="0.9" fill="currentColor"/></svg>',
  error:
    '<svg viewBox="0 0 16 16" width="16" height="16"><circle cx="8" cy="8" r="6" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M5.5 5.5l5 5M10.5 5.5l-5 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>',
}

function meta(type: ToastType): { label: string; color: string } {
  return TOAST_TYPE_META[type]
}

function onClose(item: ToastItem): void {
  dismiss(item.id)
}
</script>

<template>
  <div class="rf-toast-wrap" aria-live="polite">
    <TransitionGroup name="rf-toast">
      <div v-for="item in toasts" :key="item.id" class="rf-toast" :class="`rf-toast-${item.type}`">
        <span
          class="rf-toast-icon"
          :style="{ color: meta(item.type).color }"
          v-html="ICONS[item.type]"
        ></span>
        <div class="rf-toast-body">
          <div class="rf-toast-title">{{ item.title }}</div>
          <div v-if="item.message" class="rf-toast-message">{{ item.message }}</div>
        </div>
        <button
          class="rf-toast-close"
          type="button"
          :aria-label="`关闭：${item.title}`"
          @click="onClose(item)"
        >
          <svg viewBox="0 0 12 12" width="12" height="12">
            <path
              d="M2.5 2.5l7 7M9.5 2.5l-7 7"
              stroke="currentColor"
              stroke-width="1.5"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.rf-toast-wrap {
  position: fixed;
  top: 16px;
  right: 16px;
  z-index: 1000;
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 340px;
  max-width: calc(100vw - 32px);
  pointer-events: none;
}

.rf-toast {
  pointer-events: auto;
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px 12px 12px 14px;
  border-radius: 8px;
  background: var(--rf-bg-panel, #111827);
  border: 1px solid var(--rf-border, #1f2937);
  border-left-width: 3px;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
}

.rf-toast-success {
  border-left-color: #22c55e;
}
.rf-toast-info {
  border-left-color: #3b82f6;
}
.rf-toast-warning {
  border-left-color: #f59e0b;
}
.rf-toast-error {
  border-left-color: #ef4444;
}

.rf-toast-icon {
  flex: none;
  margin-top: 2px;
  display: inline-flex;
}

.rf-toast-body {
  flex: 1;
  min-width: 0;
}

.rf-toast-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--rf-text, #f9fafb);
  line-height: 1.4;
}

.rf-toast-message {
  margin-top: 2px;
  font-size: 12px;
  color: var(--rf-text-secondary, #9ca3af);
  line-height: 1.45;
  word-break: break-all;
}

.rf-toast-close {
  flex: none;
  border: none;
  background: none;
  padding: 2px;
  margin-top: 1px;
  cursor: pointer;
  color: var(--rf-text-muted, #6b7280);
  border-radius: 4px;
  line-height: 1;
}

.rf-toast-close:hover {
  color: var(--rf-text, #f9fafb);
  background: rgba(255, 255, 255, 0.08);
}

.rf-toast-enter-active,
.rf-toast-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.rf-toast-enter-from {
  opacity: 0;
  transform: translateX(24px);
}

.rf-toast-leave-to {
  opacity: 0;
  transform: translateX(24px);
}
</style>
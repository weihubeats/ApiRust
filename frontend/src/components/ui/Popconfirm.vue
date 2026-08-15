<script setup lang="ts">
/**
 * Popconfirm：危险操作确认气泡（替代原生 confirm）。
 * - 触发区为默认插槽；点击展开气泡，外部点击 / Esc 关闭；
 * - Teleport + 定位测量，底部空间不足自动向上翻转；
 * - 确认按钮支持 danger 样式。
 */
import { computed, onBeforeUnmount, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    title?: string
    confirmText?: string
    cancelText?: string
    danger?: boolean
    disabled?: boolean
  }>(),
  { title: '确定执行该操作吗？', confirmText: '确定', cancelText: '取消', danger: true, disabled: false },
)

const emit = defineEmits<{
  confirm: []
  cancel: []
}>()

const open = ref(false)
const triggerEl = ref<HTMLElement | null>(null)
const popEl = ref<HTMLElement | null>(null)
const pos = ref<{ left: number; top: number; up: boolean }>({ left: 0, top: 0, up: false })

const style = computed(() => ({
  left: `${pos.value.left}px`,
  top: `${pos.value.top}px`,
}))

function measure(): void {
  const el = triggerEl.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const height = 96
  const up = window.innerHeight - rect.bottom - 8 < height && rect.top > height
  pos.value = {
    left: Math.min(rect.left, window.innerWidth - 260),
    top: up ? rect.top - height - 8 : rect.bottom + 8,
    up,
  }
}

function toggle(): void {
  if (props.disabled) return
  if (open.value) close()
  else {
    measure()
    open.value = true
  }
}

function close(): void {
  open.value = false
}

function onConfirm(): void {
  close()
  emit('confirm')
}

function onCancel(): void {
  close()
  emit('cancel')
}

function onDocMouseDown(event: MouseEvent): void {
  const target = event.target as Node
  if (triggerEl.value?.contains(target) || popEl.value?.contains(target)) return
  close()
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') close()
}

watch(open, (isOpen) => {
  if (isOpen) {
    document.addEventListener('mousedown', onDocMouseDown, true)
    document.addEventListener('keydown', onKeydown)
  } else {
    document.removeEventListener('mousedown', onDocMouseDown, true)
    document.removeEventListener('keydown', onKeydown)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onDocMouseDown, true)
  document.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <span ref="triggerEl" class="pc-trigger" @click.stop="toggle">
    <slot />
    <Teleport to="body">
      <div
        v-if="open"
        ref="popEl"
        class="pc-pop"
        :class="{ up: pos.up }"
        :style="style"
        role="alertdialog"
        @click.stop
      >
        <p class="pc-title">{{ title }}</p>
        <div class="pc-actions">
          <button class="rf-btn rf-btn-sm" type="button" @click="onCancel">{{ cancelText }}</button>
          <button
            class="rf-btn rf-btn-sm"
            :class="danger ? 'rf-btn-danger' : 'rf-btn-primary'"
            type="button"
            autofocus
            @click="onConfirm"
          >
            {{ confirmText }}
          </button>
        </div>
      </div>
    </Teleport>
  </span>
</template>

<style scoped>
.pc-trigger {
  display: inline-flex;
}

.pc-pop {
  position: fixed;
  z-index: 200;
  width: 252px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow-lg);
  padding: 10px 12px;
  animation: pc-in 120ms var(--ease);
  transform-origin: top center;
}
.pc-pop.up {
  transform-origin: bottom center;
}

.pc-title {
  margin: 0 0 10px;
  font-size: 12.5px;
  color: var(--text-1);
  word-break: break-all;
}

.pc-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
}

@keyframes pc-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.97);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>
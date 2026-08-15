<script setup lang="ts">
/**
 * Modal：居中模态弹层。
 * - v-model:open 受控；Esc / 遮罩点击 / 右上 ✕ 关闭；
 * - open 后自动聚焦首个可聚焦元素（autofocus 默认开启）；
 * - 打开期间锁定 body 滚动；Teleport 到 body，双主题。
 */
import { nextTick, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    title?: string
    width?: string
    closable?: boolean
    autofocus?: boolean
  }>(),
  { title: '', width: '420px', closable: true, autofocus: true },
)

const emit = defineEmits<{
  'update:open': [open: boolean]
  close: []
}>()

const dialogEl = ref<HTMLElement | null>(null)

function close(): void {
  emit('update:open', false)
  emit('close')
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault()
    close()
  }
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      document.body.style.overflow = 'hidden'
      document.addEventListener('keydown', onKeydown)
    } else {
      document.body.style.overflow = ''
      document.removeEventListener('keydown', onKeydown)
    }
    if (open && props.autofocus) {
      nextTick(() => {
        dialogEl.value
          ?.querySelector<HTMLElement>('input, textarea, select, button')
          ?.focus()
      })
    }
  },
)
</script>

<template>
  <Teleport to="body">
    <Transition name="m">
      <div v-if="open" class="m-mask" @mousedown.self="closable && close()">
        <div
          ref="dialogEl"
          class="m-dialog"
          role="dialog"
          aria-modal="true"
          :style="{ width }"
          @mousedown.stop
        >
          <div v-if="title" class="m-head">
            <h3 class="m-title">{{ title }}</h3>
            <IconButton v-if="closable" name="x" :size="14" title="关闭" @click="close" />
          </div>
          <div class="m-body">
            <slot />
          </div>
          <div v-if="$slots.footer" class="m-foot">
            <slot name="footer" />
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.m-mask {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  background: var(--mask);
}

.m-dialog {
  display: flex;
  flex-direction: column;
  max-width: 92vw;
  max-height: 84vh;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  outline: none;
}

.m-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px 0;
}

.m-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-1);
}

.m-body {
  padding: 12px 16px 16px;
  overflow-y: auto;
  color: var(--text-2);
  font-size: 13px;
}

.m-foot {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 0 16px 14px;
}

.m-enter-active,
.m-leave-active {
  transition: opacity 160ms var(--ease);
}
.m-enter-active .m-dialog,
.m-leave-active .m-dialog {
  transition: transform 160ms var(--ease);
}
.m-enter-from,
.m-leave-to {
  opacity: 0;
}
.m-enter-from .m-dialog,
.m-leave-to .m-dialog {
  transform: translateY(-10px) scale(0.98);
}
</style>
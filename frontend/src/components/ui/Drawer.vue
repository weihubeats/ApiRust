<script setup lang="ts">
/**
 * Drawer：右侧侧滑抽屉。
 * - v-model:open 受控；Esc / 遮罩点击 / 右上 ✕ 关闭；
 * - 打开期间锁定 body 滚动；Teleport 到 body。
 */
import { onBeforeUnmount, ref, watch } from 'vue'
import IconButton from './IconButton.vue'

const props = withDefaults(
  defineProps<{
    open: boolean
    title?: string
    width?: string
    closable?: boolean
  }>(),
  { title: '', width: '640px', closable: true },
)

const emit = defineEmits<{
  'update:open': [open: boolean]
  close: []
}>()

const bodyEl = ref<HTMLElement | null>(null)

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
  },
)

onBeforeUnmount(() => {
  document.body.style.overflow = ''
  document.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <Teleport to="body">
    <Transition name="d">
      <div v-if="open" class="d-mask" @mousedown.self="closable && close()">
        <aside
          ref="bodyEl"
          class="d-drawer"
          role="dialog"
          aria-modal="true"
          :style="{ width }"
          @mousedown.stop
        >
          <div class="d-head">
            <h3 class="d-title">{{ title }}</h3>
            <IconButton v-if="closable" name="x" :size="14" title="关闭" @click="close" />
          </div>
          <div class="d-body">
            <slot />
          </div>
        </aside>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.d-mask {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  justify-content: flex-end;
  background: var(--mask);
}

.d-drawer {
  display: flex;
  flex-direction: column;
  height: 100%;
  max-width: 96vw;
  border-left: 1px solid var(--border-strong);
  border-radius: var(--radius-lg) 0 0 var(--radius-lg);
  background: var(--bg-elevated);
  box-shadow: -16px 0 48px rgba(0, 0, 0, 0.4);
  overflow: hidden;
}

.d-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.d-title {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: 600;
  color: var(--text-1);
}

.d-body {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 16px;
}

.d-enter-active,
.d-leave-active {
  transition: transform 220ms var(--ease), opacity 220ms var(--ease);
}

.d-mask {
  transition: background 220ms var(--ease);
}

.d-enter-from,
.d-leave-to {
  background: transparent;
}

.d-enter-from .d-drawer,
.d-leave-to .d-drawer {
  transform: translateX(100%);
}
</style>
<script setup lang="ts">
/**
 * Tooltip：hover 提示（250ms 延迟出现，随触发元素定位）。
 * 触发元素为默认插槽；气泡 fixed 定位避免被 overflow 裁剪。
 */
import { onBeforeUnmount, ref } from 'vue'
import { onMounted } from 'vue'

const props = withDefaults(
  defineProps<{ content: string; placement?: 'top' | 'bottom' }>(),
  { content: '', placement: 'top' },
)

const visible = ref(false)
const triggerEl = ref<HTMLElement | null>(null)
const tipEl = ref<HTMLElement | null>(null)
const pos = ref({ left: 0, top: 0 })
let timer: number | null = null

function show(): void {
  if (!props.content) return
  timer = window.setTimeout(() => {
    position()
    visible.value = true
  }, 250)
}

function hide(): void {
  if (timer !== null) {
    window.clearTimeout(timer)
    timer = null
  }
  visible.value = false
}

function position(): void {
  const el = triggerEl.value
  const tip = tipEl.value
  if (!el || !tip) return
  const rect = el.getBoundingClientRect()
  const tw = tip.offsetWidth
  const th = tip.offsetHeight
  const left = rect.left + rect.width / 2 - tw / 2
  const top =
    props.placement === 'top'
      ? rect.top - th - 6
      : rect.bottom + 6
  pos.value = {
    left: Math.max(4, Math.min(left, window.innerWidth - tw - 4)),
    top,
  }
}

function onReposition(): void {
  if (visible.value) position()
}

onMounted(() => {
  window.addEventListener('scroll', onReposition, true)
  window.addEventListener('resize', onReposition)
})

onBeforeUnmount(() => {
  window.removeEventListener('scroll', onReposition, true)
  window.removeEventListener('resize', onReposition)
})
</script>

<template>
  <span
    ref="triggerEl"
    class="tt-trigger"
    @mouseenter="show"
    @mouseleave="hide"
    @focusin="show"
    @focusout="hide"
  >
    <slot />
    <Teleport to="body">
      <span
        v-if="visible"
        ref="tipEl"
        class="tt-tip"
        :class="placement"
        :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
        role="tooltip"
      >
        {{ content }}
      </span>
    </Teleport>
  </span>
</template>

<style scoped>
.tt-trigger {
  display: inline-flex;
}

.tt-tip {
  position: fixed;
  z-index: 300;
  max-width: 260px;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
  background: var(--text-1);
  color: var(--bg-app);
  font-size: 11.5px;
  line-height: 1.5;
  pointer-events: none;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  box-shadow: var(--shadow);
  animation: tt-in 120ms var(--ease);
}
.tt-tip.bottom {
  animation-name: tt-in-bottom;
}

@keyframes tt-in {
  from {
    opacity: 0;
    transform: translateY(3px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
@keyframes tt-in-bottom {
  from {
    opacity: 0;
    transform: translateY(-3px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
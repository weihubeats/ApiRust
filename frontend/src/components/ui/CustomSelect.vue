<script setup lang="ts">
/**
 * CustomSelect：自绘下拉选择器。
 * - 触发按钮 + Teleport 到 body 的浮层（避免被 overflow 裁剪），滚动/尺寸变化自动收起；
 * - 键盘：↑/↓ 移动高亮（循环）、Enter 选中、Esc 关闭；外部点击关闭；
 * - 五态：default / hover / focus / active(open) / disabled × 深/浅主题；
 * - 作用域插槽 #display 定制触发区文案（如方法着色）、#option 定制选项行。
 */
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import Icon from './Icon.vue'

export interface SelectOption {
  value: string | number
  label: string
}

const props = withDefaults(
  defineProps<{
    modelValue?: string | number | null
    options: SelectOption[]
    placeholder?: string
    disabled?: boolean
    size?: 'sm' | 'md'
    popClass?: string
  }>(),
  { modelValue: null, placeholder: '请选择…', disabled: false, size: 'md', popClass: '' },
)

const emit = defineEmits<{
  'update:modelValue': [value: string | number]
  change: [value: string | number]
}>()

const open = ref(false)
const highlight = ref(-1)
const triggerEl = ref<HTMLButtonElement | null>(null)
const popupEl = ref<HTMLDivElement | null>(null)
const pos = ref<{ left: number; top: number; width: number; up: boolean }>({
  left: 0,
  top: 0,
  width: 0,
  up: false,
})

const selectedIndex = computed(() => {
  const idx = props.options.findIndex((o) => String(o.value) === String(props.modelValue))
  return idx === -1 ? -1 : idx
})

const displayLabel = computed(() => {
  const o = props.options[selectedIndex.value]
  return o ? o.label : ''
})

function measure(): void {
  const el = triggerEl.value
  if (!el) return
  const rect = el.getBoundingClientRect()
  const height = Math.min(props.options.length * 30 + 8, 280)
  const spaceBelow = window.innerHeight - rect.bottom - 8
  const up = spaceBelow < height && rect.top > height
  pos.value = {
    left: rect.left,
    top: up ? rect.top - height - 4 : rect.bottom + 4,
    width: rect.width,
    up,
  }
}

function openPopup(): void {
  if (props.disabled) return
  measure()
  open.value = true
  highlight.value = selectedIndex.value
}

function close(): void {
  open.value = false
}

function pick(option: SelectOption): void {
  emit('update:modelValue', option.value)
  emit('change', option.value)
  close()
}

function onKeydown(event: KeyboardEvent): void {
  if (props.disabled) return
  if (event.key === 'ArrowDown') {
    event.preventDefault()
    if (!open.value) {
      openPopup()
      return
    }
    highlight.value = (highlight.value + 1) % props.options.length
    scrollToHighlight()
  } else if (event.key === 'ArrowUp') {
    event.preventDefault()
    if (!open.value) {
      openPopup()
      return
    }
    highlight.value = (highlight.value - 1 + props.options.length) % props.options.length
    scrollToHighlight()
  } else if (event.key === 'Enter') {
    event.preventDefault()
    if (open.value && highlight.value >= 0) {
      pick(props.options[highlight.value])
    } else {
      openPopup()
    }
  } else if (event.key === 'Escape') {
    event.preventDefault()
    close()
  } else if (event.key === 'Tab') {
    close()
  }
}

function scrollToHighlight(): void {
  requestAnimationFrame(() => {
    const el = popupEl.value?.querySelector<HTMLElement>('.cs-opt.hl')
    el?.scrollIntoView({ block: 'nearest' })
  })
}

function onDocMouseDown(event: MouseEvent): void {
  const target = event.target as Node
  if (triggerEl.value?.contains(target) || popupEl.value?.contains(target)) return
  close()
}

function onReposition(): void {
  if (open.value) measure()
}

watch(open, (isOpen) => {
  if (isOpen) {
    document.addEventListener('mousedown', onDocMouseDown, true)
    window.addEventListener('scroll', onReposition, true)
    window.addEventListener('resize', onReposition)
  } else {
    document.removeEventListener('mousedown', onDocMouseDown, true)
    window.removeEventListener('scroll', onReposition, true)
    window.removeEventListener('resize', onReposition)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onDocMouseDown, true)
  window.removeEventListener('scroll', onReposition, true)
  window.removeEventListener('resize', onReposition)
})
</script>

<template>
  <div class="cs" :class="[`size-${size}`, { open, disabled }]">
    <button
      ref="triggerEl"
      type="button"
      class="cs-trigger"
      :disabled="disabled"
      aria-haspopup="listbox"
      :aria-expanded="open"
      @click="open ? close() : openPopup()"
      @keydown="onKeydown"
    >
      <span class="cs-value" :class="{ 'is-empty': !displayLabel }">
        <slot name="display" :label="displayLabel" :selected="options[selectedIndex] ?? null">
          {{ displayLabel || placeholder }}
        </slot>
      </span>
      <Icon class="cs-caret" :name="open ? 'chevron-up' : 'chevron-down'" :size="12" />
    </button>

    <Teleport to="body">
      <div
        v-if="open"
        ref="popupEl"
        class="cs-pop"
        :class="[popClass, { up: pos.up }]"
        :style="{ left: `${pos.left}px`, top: `${pos.top}px`, width: `${pos.width}px` }"
        role="listbox"
      >
        <div
          v-for="(o, i) in options"
          :key="String(o.value)"
          class="cs-opt"
          :class="{ hl: highlight === i, sel: String(o.value) === String(modelValue) }"
          role="option"
          :aria-selected="String(o.value) === String(modelValue)"
          @click="pick(o)"
          @mouseenter="highlight = i"
        >
          <span class="cs-opt-check">
            <Icon v-if="String(o.value) === String(modelValue)" name="check" :size="12" />
          </span>
          <span class="cs-opt-label">
            <slot name="option" :option="o" :selected="String(o.value) === String(modelValue)">
              {{ o.label }}
            </slot>
          </span>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.cs {
  position: relative;
  display: inline-flex;
  flex: 0 0 auto;
}

.cs-trigger {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-1);
  border-radius: var(--radius);
  font-size: 13px;
  font-family: var(--font-mono);
  padding: 0 8px 0 10px;
  cursor: pointer;
  user-select: none;
  transition:
    background var(--dur) var(--ease),
    border-color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.cs.size-md .cs-trigger {
  height: var(--h-md);
}
.cs.size-sm .cs-trigger {
  height: var(--h-sm);
  font-size: 12px;
}
.cs-trigger:hover:not(:disabled) {
  background: var(--bg-elevated);
  border-color: var(--border-strong);
}
.cs-trigger:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.cs.open .cs-trigger {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint);
}
.cs.disabled .cs-trigger {
  opacity: 0.45;
  cursor: default;
}

.cs-value {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: left;
  display: inline-flex;
  align-items: center;
}
.cs-value.is-empty {
  color: var(--text-3);
}

.cs-caret {
  flex-shrink: 0;
  color: var(--text-3);
  transition: transform var(--dur) var(--ease);
}

.cs-pop {
  position: fixed;
  z-index: 1000;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  box-shadow: var(--shadow);
  padding: 4px;
  max-height: 280px;
  overflow-y: auto;
  transform-origin: top;
  animation: cs-in 120ms var(--ease);
}
.cs-pop.up {
  transform-origin: bottom;
}

.cs-opt {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 30px;
  padding: 0 10px 0 8px;
  cursor: pointer;
  font-size: 12.5px;
  font-family: var(--font-mono);
  color: var(--text-1);
  white-space: nowrap;
  overflow: hidden;
  user-select: none;
  transition: background var(--dur) var(--ease);
}
.cs-opt.hl {
  background: var(--bg-hover);
}
.cs-opt.sel {
  color: var(--accent);
}
.cs-opt:active {
  background: var(--accent-tint);
}

.cs-opt-check {
  width: 14px;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--accent);
}

.cs-opt-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@keyframes cs-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>

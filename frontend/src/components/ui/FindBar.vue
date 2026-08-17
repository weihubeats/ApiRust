<script setup lang="ts">
/**
 * FindBar：响应内查找条（Find in Response）。
 * 输入即搜；Enter = 下一个，Shift + Enter = 上一个，Esc = 关闭。
 * 打开时自动聚焦输入框。
 */
import { computed, onMounted, ref } from 'vue'
import Icon from './Icon.vue'

const props = defineProps<{
  query: string
  index: number
  total: number
}>()

const emit = defineEmits<{
  'update:query': [string]
  prev: []
  next: []
  close: []
}>()

/** 输入值代理：写入时上报给父级（响应式 v-model 语义）。 */
const inputValue = computed({
  get: () => props.query,
  set: (v: string) => emit('update:query', v),
})

const input = ref<HTMLInputElement | null>(null)

onMounted(() => {
  input.value?.focus()
})

function onInputKeydown(e: KeyboardEvent): void {
  if (e.key === 'Enter') {
    e.preventDefault()
    if (e.shiftKey) emit('prev')
    else emit('next')
  } else if (e.key === 'Escape') {
    e.preventDefault()
    emit('close')
  }
}
</script>

<template>
  <div class="findbar">
    <Icon name="search" :size="13" class="findbar-icon" />
    <input
      ref="input"
      v-model="inputValue"
      class="findbar-input"
      spellcheck="false"
      placeholder="在响应中查找…"
      @keydown="onInputKeydown"
    />
    <span class="findbar-count" :class="{ empty: total === 0 }">
      {{ total ? `${index + 1} / ${total}` : '无匹配' }}
    </span>
    <button
      class="findbar-btn"
      type="button"
      title="上一个 (Shift+Enter)"
      :disabled="total === 0"
      @click="emit('prev')"
    >
      <Icon name="chevron-up" :size="13" />
    </button>
    <button
      class="findbar-btn"
      type="button"
      title="下一个 (Enter)"
      :disabled="total === 0"
      @click="emit('next')"
    >
      <Icon name="chevron-down" :size="13" />
    </button>
    <button class="findbar-btn findbar-close" type="button" title="关闭查找 (Esc)" @click="emit('close')">
      <Icon name="x" :size="13" />
    </button>
  </div>
</template>

<style scoped>
.findbar {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
}

.findbar-icon {
  flex-shrink: 0;
  color: var(--text-3);
}

.findbar-input {
  flex: 1 1 auto;
  min-width: 0;
  padding: 4px 8px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-input);
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-1);
  outline: none;
}
.findbar-input:focus {
  border-color: var(--accent);
}

.findbar-count {
  flex-shrink: 0;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-2);
  min-width: 52px;
  text-align: right;
}
.findbar-count.empty {
  color: var(--text-3);
}

.findbar-btn {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: none;
  border-radius: 6px;
  background: none;
  color: var(--text-2);
  cursor: pointer;
}
.findbar-btn:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text-1);
}
.findbar-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.findbar-close:hover {
  color: var(--danger);
}
</style>
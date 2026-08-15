<script setup lang="ts">
/**
 * Tabs：标签页导航。激活 = accent 文本 + 2px 下划线；支持数量徽标。
 */
export interface TabItem {
  key: string
  label: string
  count?: number
  disabled?: boolean
}

withDefaults(
  defineProps<{
    modelValue?: string
    tabs: TabItem[]
    size?: 'sm' | 'md'
  }>(),
  { modelValue: '', size: 'md' },
)

const emit = defineEmits<{
  'update:modelValue': [key: string]
  change: [key: string]
}>()

function pick(key: string): void {
  emit('update:modelValue', key)
  emit('change', key)
}
</script>

<template>
  <div class="tabs" :class="`size-${size}`" role="tablist">
    <button
      v-for="t in tabs"
      :key="t.key"
      type="button"
      class="tab"
      :class="{ active: modelValue === t.key }"
      role="tab"
      :aria-selected="modelValue === t.key"
      :disabled="t.disabled"
      @click="pick(t.key)"
    >
      <span class="tab-label">{{ t.label }}</span>
      <span v-if="t.count !== undefined" class="tab-badge" :class="{ on: modelValue === t.key }">
        {{ t.count }}
      </span>
    </button>
  </div>
</template>

<style scoped>
.tabs {
  display: flex;
  align-items: stretch;
  gap: 2px;
  overflow-x: auto;
  overflow-y: hidden;
  border-bottom: 1px solid var(--border);
}
.tabs.size-md .tab {
  height: 34px;
  padding: 0 12px;
  font-size: 13px;
}
.tabs.size-sm .tab {
  height: 30px;
  padding: 0 10px;
  font-size: 12.5px;
}

.tab {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: none;
  background: none;
  color: var(--text-2);
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
  transition:
    color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.tab:hover:not(:disabled) {
  color: var(--text-1);
  background: var(--bg-hover);
}
.tab:active:not(:disabled) {
  background: var(--bg-active);
}
.tab:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: -2px;
}
.tab.active {
  color: var(--accent);
  font-weight: 600;
}
.tab.active::after {
  content: '';
  position: absolute;
  left: 10px;
  right: 10px;
  bottom: 0;
  height: 2px;
  border-radius: 1px;
  background: var(--accent);
}
.tab:disabled {
  opacity: 0.4;
  cursor: default;
}

.tab-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 17px;
  height: 17px;
  padding: 0 5px;
  border-radius: 999px;
  font-size: 10.5px;
  font-weight: 600;
  line-height: 1;
  color: var(--text-2);
  background: var(--bg-hover);
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease);
}
.tab-badge.on {
  color: #fff;
  background: var(--accent);
}
</style>
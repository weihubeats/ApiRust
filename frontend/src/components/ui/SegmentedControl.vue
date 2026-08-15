<script setup lang="ts">
/**
 * SegmentedControl：分段控件（Body 类型等）。激活项为填充 pill。
 * 五态：default / hover / focus / active / disabled × 双主题。
 */
import Icon, { type IconName } from './Icon.vue'

export interface SegmentOption {
  value: string
  label: string
  icon?: IconName
}

withDefaults(
  defineProps<{
    modelValue?: string | null
    options: SegmentOption[]
    disabled?: boolean
    size?: 'sm' | 'md'
  }>(),
  { modelValue: null, disabled: false, size: 'md' },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
  change: [value: string]
}>()

function pick(value: string): void {
  emit('update:modelValue', value)
  emit('change', value)
}
</script>

<template>
  <div class="seg" :class="[`size-${size}`, { disabled }]" role="tablist">
    <button
      v-for="o in options"
      :key="o.value"
      type="button"
      class="seg-item"
      :class="{ active: String(modelValue) === o.value }"
      role="tab"
      :aria-selected="String(modelValue) === o.value"
      :disabled="disabled"
      @click="pick(o.value)"
    >
      <Icon v-if="o.icon" :name="o.icon" :size="13" />{{ o.label }}
    </button>
  </div>
</template>

<style scoped>
.seg {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius);
}
.seg.size-md .seg-item {
  height: 26px;
  padding: 0 14px;
  font-size: 12.5px;
}
.seg.size-sm .seg-item {
  height: 22px;
  padding: 0 10px;
  font-size: 12px;
}

.seg-item {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: none;
  background: transparent;
  color: var(--text-2);
  border-radius: var(--radius-sm);
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
  user-select: none;
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.seg-item:hover:not(:disabled) {
  color: var(--text-1);
  background: var(--bg-hover);
}
.seg-item:active:not(:disabled) {
  background: var(--bg-active);
}
.seg-item:focus-visible {
  outline: 2px solid var(--focus-ring);
  outline-offset: -2px;
}
.seg-item.active {
  background: var(--accent);
  color: #fff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
}
.seg-item.active:hover {
  background: var(--accent-hover);
}
.seg.disabled .seg-item {
  opacity: 0.45;
  cursor: default;
}
</style>
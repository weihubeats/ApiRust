<script setup lang="ts">
/**
 * Avatar：圆角方块头像，hash 取色（6 色板）+ 首字符。
 */
const PALETTE = ['#3b82f6', '#8b5cf6', '#10b981', '#f59e0b', '#ec4899', '#06b6d4']

const props = withDefaults(defineProps<{ name: string; size?: number }>(), { size: 32 })

function hashColor(): string {
  let sum = 0
  for (const ch of props.name) sum += ch.codePointAt(0) ?? 0
  return PALETTE[sum % PALETTE.length]
}

function initial(): string {
  const name = props.name.trim()
  return name ? name.slice(0, 1).toUpperCase() : '?'
}
</script>

<template>
  <span
    class="av"
    :style="{
      width: `${size}px`,
      height: `${size}px`,
      borderRadius: `${Math.max(4, Math.round(size / 4))}px`,
      background: hashColor(),
      fontSize: `${Math.round(size * 0.44)}px`,
    }"
    aria-hidden="true"
  >
    {{ initial() }}
  </span>
</template>

<style scoped>
.av {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: #fff;
  font-family: var(--font-ui);
  font-weight: 600;
  line-height: 1;
  user-select: none;
}
</style>
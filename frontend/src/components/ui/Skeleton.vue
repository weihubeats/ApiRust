<script setup lang="ts">
/**
 * Skeleton：加载骨架（shimmer 动画，双主题跟随令牌）。
 */
withDefaults(
  defineProps<{
    lines?: number
    width?: string
    height?: string
    radius?: string
  }>(),
  { lines: 1, width: '100%', height: '14px', radius: 'var(--radius-sm)' },
)
</script>

<template>
  <div class="sk" :style="{ gap: `${Math.round(lines > 1 ? 8 : 0)}px` }">
    <div
      v-for="i in lines"
      :key="i"
      class="sk-bar"
      :style="{ width, height, borderRadius: radius }"
    ></div>
  </div>
</template>

<style scoped>
.sk {
  display: flex;
  flex-direction: column;
}
.sk-bar {
  background: linear-gradient(
    100deg,
    var(--bg-hover) 30%,
    var(--bg-active) 50%,
    var(--bg-hover) 70%
  );
  background-size: 200% 100%;
  animation: sk-shimmer 1.4s ease-in-out infinite;
  flex-shrink: 0;
}
@keyframes sk-shimmer {
  from {
    background-position: 200% 0;
  }
  to {
    background-position: -200% 0;
  }
}
</style>
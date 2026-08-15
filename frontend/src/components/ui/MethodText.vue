<script setup lang="ts">
/**
 * MethodText：HTTP 方法彩色文本标识（mono、加粗、语义色；双主题跟随令牌）。
 */
withDefaults(defineProps<{ method: string }>(), {})

const TONES: Record<string, string> = {
  GET: 'get',
  POST: 'post',
  PUT: 'put',
  PATCH: 'patch',
  DELETE: 'delete',
  HEAD: 'get',
  OPTIONS: 'options',
  GRAPHQL: 'graphql',
}
</script>

<template>
  <span class="mt" :class="`rf-method-${TONES[method.toUpperCase()] ?? 'options'}`">{{ method }}</span>
</template>

<style scoped>
.mt {
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.03em;
  line-height: 1.5;
  white-space: nowrap;
}
</style>
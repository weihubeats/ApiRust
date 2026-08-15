<script setup lang="ts">
/**
 * Brand：左上角品牌区（logo + 标题），点击返回主页。
 * 标题支持省略号截断，深/浅双主题，hover/focus 五态。
 */
import { useRouter } from 'vue-router'

withDefaults(defineProps<{ title: string; subtitle?: string }>(), { subtitle: '' })

const router = useRouter()
</script>

<template>
  <button type="button" class="brand" title="回到项目首页" @click="router.push('/projects')">
    <span class="brand-logo" aria-hidden="true">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
        <path d="M13.2 2 4.4 13.6h6.2L9.1 22l8.9-11.6h-6.3L13.2 2z" fill="currentColor" />
      </svg>
    </span>
    <span class="brand-text">
      <span class="brand-title">{{ title }}</span>
      <span v-if="subtitle" class="brand-subtitle">{{ subtitle }}</span>
    </span>
  </button>
</template>

<style scoped>
.brand {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
  flex: 1;
  padding: 4px 6px;
  border: none;
  background: none;
  border-radius: var(--radius);
  cursor: pointer;
  text-align: left;
  transition: background var(--dur) var(--ease);
}
.brand:hover {
  background: var(--bg-hover);
}
.brand:active {
  background: var(--bg-hover);
}
.brand:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

.brand-logo {
  flex-shrink: 0;
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--accent) 0%, var(--put) 100%);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.25);
}

.brand-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  gap: 0;
}

.brand-title {
  font-size: 13px;
  font-weight: 600;
  line-height: 1.3;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.brand-subtitle {
  font-size: 10.5px;
  line-height: 1.3;
  color: var(--text-3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

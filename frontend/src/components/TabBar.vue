<script setup lang="ts">
/**
 * TabBar：打开中的接口标签页（草稿名 + 脏标记 ● + 关闭）。
 */
import { useWorkspaceStore } from '../stores/workspace'

const store = useWorkspaceStore()

function close(id: string): void {
  if (store.isDirty(id) && !window.confirm('该接口有未保存的修改，确认关闭？')) return
  store.closeTab(id)
}
</script>

<template>
  <div class="tab-bar">
    <div
      v-for="id in store.openTabs"
      :key="id"
      class="tab"
      :class="{ active: store.activeTabId === id }"
      @click="store.activeTabId = id"
    >
      <span class="tab-title">{{ store.titleOf(id) }}</span>
      <span v-if="store.isDirty(id)" class="tab-dirty" title="未保存">●</span>
      <button class="tab-close" type="button" title="关闭" @click.stop="close(id)">✕</button>
    </div>
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  gap: 2px;
  padding: 4px 4px 0;
  overflow-x: auto;
  border-bottom: 1px solid var(--rf-border);
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-radius: 8px 8px 0 0;
  border: 1px solid transparent;
  border-bottom: none;
  font-size: 12.5px;
  color: var(--rf-text-secondary);
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
  max-width: 220px;
}

.tab:hover {
  background: var(--rf-hover);
}

.tab.active {
  background: var(--rf-bg-panel-2);
  border-color: var(--rf-border);
  color: var(--rf-text);
}

.tab-title {
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-dirty {
  color: var(--rf-warning);
  font-size: 10px;
}

.tab-close {
  border: none;
  background: none;
  color: var(--rf-text-muted);
  font-size: 11px;
  padding: 0 2px;
  border-radius: 4px;
  cursor: pointer;
  line-height: 1;
}

.tab-close:hover {
  color: var(--rf-danger);
  background: var(--rf-danger-tint);
}
</style>
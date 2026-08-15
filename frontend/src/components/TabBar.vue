<script setup lang="ts">
/**
 * TabBar：打开中的接口标签页。
 * - 每个标签：方法标签（GET 绿 / POST 黄…）+ 截断的接口名；宽 120–200px；
 * - 激活态 = text-1 + 底部主题色下划线；未保存草稿在标题旁显示小圆点，
 *   hover 时圆点被 ✕ 替换（两者不同时出现，避免挤占）；
 * - 关闭按钮 hover 才出现；脏标签关闭走 Popconfirm。
 */
import { useWorkspaceStore } from '../stores/workspace'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import Popconfirm from './ui/Popconfirm.vue'
import Tooltip from './ui/Tooltip.vue'

const store = useWorkspaceStore()

function close(id: string): void {
  store.closeTab(id)
}

function methodOf(id: string): string {
  return store.draftOf(id)?.method ?? 'GET'
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
      <span class="method-tag" :class="`mt-${methodOf(id).toLowerCase()}`">{{ methodOf(id) }}</span>
      <span class="tab-title" :title="store.titleOf(id)">{{ store.titleOf(id) }}</span>
      <span v-if="store.isDirty(id)" class="tab-dirty" title="未保存"><Icon name="dot" :size="7" /></span>
      <Popconfirm
        v-if="store.isDirty(id)"
        title="该接口有未保存的修改，确认关闭？"
        @confirm="close(id)"
      >
        <IconButton class="tab-close" name="x" :size="12" title="关闭" />
      </Popconfirm>
      <IconButton v-else class="tab-close" name="x" :size="12" title="关闭" @click.stop="close(id)" />
    </div>
    <Tooltip content="新建请求 (Cmd+T)">
      <button
        class="tab-add"
        type="button"
        aria-label="新建请求"
        @click="store.openNewEndpoint(null)"
      >
        <Icon name="plus" :size="15" />
      </button>
    </Tooltip>
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  gap: 2px;
  padding: 6px 8px 0;
  overflow-x: auto;
  overflow-y: hidden;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
  flex-shrink: 0;
}

.tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  min-width: 120px;
  max-width: 200px;
  padding: 0 4px 0 10px;
  border-radius: var(--radius) var(--radius) 0 0;
  position: relative;
  font-size: 12.5px;
  color: var(--text-2);
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
  transition:
    color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.tab:hover {
  background: var(--bg-hover);
  color: var(--text-1);
}
.tab.active {
  color: var(--text-1);
  background: var(--bg-app);
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

/* 方法标签：单色胶囊 + 方法色 */
.method-tag {
  flex-shrink: 0;
  font-family: var(--font-mono);
  font-size: 10.5px;
  font-weight: 700;
  line-height: 1;
  padding: 3px 6px;
  border-radius: 999px;
}
.mt-get {
  color: var(--rf-success);
  background: var(--success-tint);
}
.mt-post {
  color: var(--rf-warning);
  background: var(--warning-tint);
}
.mt-put {
  color: var(--rf-info);
  background: var(--info-tint);
}
.mt-delete {
  color: var(--rf-danger);
  background: var(--danger-tint);
}
.mt-patch {
  color: var(--patch);
  background: var(--accent-tint);
}
.mt-head,
.mt-options {
  color: var(--rf-text-muted);
  background: var(--bg-hover);
}

.tab-title {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 未保存圆点：常态显示，hover 时被 ✕ 顶替 */
.tab-dirty {
  display: inline-flex;
  color: var(--warning);
  flex-shrink: 0;
  transition: opacity var(--dur) var(--ease);
}
.tab:hover .tab-dirty {
  opacity: 0;
}

.tab-close {
  width: 20px;
  height: 20px;
  opacity: 0;
  flex-shrink: 0;
  transition:
    opacity var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.tab:hover .tab-close {
  opacity: 1;
}
.tab-close:hover {
  background: var(--danger-tint);
  color: var(--danger);
}

/* ---- 快速新建请求「+」：28×28、圆角背景、hover 白色 10% 蒙层 ---- */
.tab-add {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  margin: 0 2px 2px 4px;
  align-self: center;
  flex-shrink: 0;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--accent);
  cursor: pointer;
  padding: 0;
  transition:
    background var(--dur) var(--ease),
    transform var(--dur) var(--ease);
}
.tab-add:hover {
  background: rgba(255, 255, 255, 0.1);
}
.tab-add:active {
  transform: scale(0.92);
}
.tab-add:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
</style>

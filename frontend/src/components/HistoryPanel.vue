<script setup lang="ts">
/**
 * HistoryPanel：请求历史面板（侧栏「请求历史」页签）。
 * - 按时间倒序展示最近请求：Method 标签 + URL Path + 状态码 + 耗时 + 时间；
 * - 「仅当前接口」过滤与「清空历史」（Popconfirm 二次确认）；
 * - 点击记录调用 store.restoreFromHistory：Method/URL/Headers/Body 恢复到主编辑器。
 * 数据源为 workspace store（发送成功后由 EndpointEditor 触发刷新）。
 */
import { onMounted } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { formatDuration } from '../utils/format'
import EmptyState from './ui/EmptyState.vue'
import IconButton from './ui/IconButton.vue'
import Popconfirm from './ui/Popconfirm.vue'

const store = useWorkspaceStore()

onMounted(() => {
  void store.loadHistories()
})

function reload(): void {
  void store.loadHistories()
}

async function clear(): Promise<void> {
  await store.clearHistories()
}

/** 展示用短地址：去掉协议与域名，仅保留 path + query。 */
function shortUrl(url: string): string {
  const stripped = url.replace(/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\/[^/]+/, '')
  return stripped || url
}

/** 本地短时间：今天显示 HH:mm，更早显示 MM-DD HH:mm。 */
function shortTime(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return ''
  const now = new Date()
  const pad = (n: number): string => String(n).padStart(2, '0')
  const hm = `${pad(d.getHours())}:${pad(d.getMinutes())}`
  const sameDay =
    d.getFullYear() === now.getFullYear() &&
    d.getMonth() === now.getMonth() &&
    d.getDate() === now.getDate()
  return sameDay ? hm : `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${hm}`
}
</script>

<template>
  <div class="history-panel">
    <div class="hp-head">
      <label class="hp-filter" title="只显示当前激活接口的请求记录">
        <input
          v-model="store.historyOnlyCurrent"
          type="checkbox"
          class="hp-check"
          @change="reload"
        />
        仅当前接口
      </label>
      <span class="hp-count">{{ store.histories.length }} 条</span>
      <Popconfirm
        title="清空请求历史？该操作不可恢复。"
        confirm-text="清空"
        danger
        @confirm="clear"
      >
        <IconButton name="trash" :size="14" tone="danger" title="清空历史记录" />
      </Popconfirm>
    </div>

    <div class="hp-list">
      <button
        v-for="h in store.histories"
        :key="h.id"
        class="hp-row"
        type="button"
        @click="store.restoreFromHistory(h)"
      >
        <span class="hp-method" :class="`m-select-${h.method.toLowerCase()}`">{{ h.method }}</span>
        <span class="hp-url" v-tooltip-overflow="h.url">{{ shortUrl(h.url) }}</span>
        <span
          v-if="h.status != null"
          class="hp-status"
          :class="{ err: (h.status ?? 0) >= 400 }"
        >
          {{ h.status }}
        </span>
        <span class="hp-meta">{{ formatDuration(h.duration_ms ?? 0) }}</span>
        <span class="hp-meta hp-time">{{ shortTime(h.created_at) }}</span>
      </button>
      <EmptyState
        v-if="!store.histories.length"
        icon="history"
        compact
        title="暂无请求历史"
        description="发送请求后，最近 50 条记录会显示在这里"
      />
    </div>
  </div>
</template>

<style scoped>
.history-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

/* ---- 工具行：过滤 + 计数 + 清空 ---- */
.hp-head {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 30px;
  padding: 0 12px;
  flex-shrink: 0;
}

.hp-filter {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11.5px;
  color: var(--text-2);
  cursor: pointer;
  user-select: none;
  white-space: nowrap;
}

.hp-check {
  accent-color: var(--accent);
}

.hp-count {
  flex: 1;
  min-width: 0;
  font-size: 11px;
  color: var(--text-3);
  text-align: right;
}

/* ---- 列表 ---- */
.hp-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 4px 8px 12px;
}

.hp-row {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  padding: 6px 6px;
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  transition: background var(--dur) var(--ease);
}

.hp-row:hover {
  background: var(--bg-hover);
}

.hp-method {
  flex-shrink: 0;
  width: 44px;
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.2px;
}

.hp-url {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hp-status {
  flex-shrink: 0;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 600;
  color: var(--success);
}

.hp-status.err {
  color: var(--danger);
}

.hp-meta {
  flex-shrink: 0;
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-3);
  white-space: nowrap;
}
</style>

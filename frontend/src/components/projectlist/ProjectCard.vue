<script setup lang="ts">
/**
 * ProjectCard：项目卡片。
 *
 * - 彩色渐变头像 / 状态标签（Active/Draft）/ API 数与时间指标；
 * - 更多菜单（重命名 / 复制 / 删除，菜单开合状态由父级统一管理）；
 * - hover 显示 Open 箭头，点击整卡进入项目。
 */
import Icon from '../ui/Icon.vue'
import IconButton from '../ui/IconButton.vue'
import type { Project } from '../../types/foxApi'
import { avatarStyle, initials, timeAgo } from './projectMeta'

defineProps<{
  project: Project
  count: number
  active: boolean
  /** 当前展开更多菜单的卡片 id（父级持有，保证同时只开一个） */
  menuOpen: boolean
}>()

const emit = defineEmits<{
  open: []
  'toggle-menu': []
  rename: []
  duplicate: []
  delete: []
}>()
</script>

<template>
  <div class="proj-card" @click="emit('open')">
    <span class="proj-avatar" :style="avatarStyle(project.name)">{{ initials(project.name) }}</span>
    <div class="proj-main">
      <div class="proj-title-row">
        <span class="proj-title" :title="project.name">{{ project.name }}</span>
        <span class="proj-status" :class="{ active }">{{ active ? 'Active' : 'Draft' }}</span>
      </div>
      <p class="proj-desc">{{ project.description || '暂无描述' }}</p>
      <div class="proj-metrics">
        <span class="metric"><Icon name="plug" :size="12" />{{ count }} APIs</span>
        <span class="metric-sep">·</span>
        <span class="metric"><Icon name="clock" :size="12" />{{ timeAgo(project.updated_at) }}</span>
      </div>
    </div>
    <div class="proj-side">
      <span class="proj-open" title="打开项目">
        <Icon name="arrow-up-right" :size="13" /> Open
      </span>
      <div class="proj-more" @click.stop>
        <IconButton name="more-horizontal" :size="16" title="更多操作" @click="emit('toggle-menu')" />
        <div v-if="menuOpen" class="more-menu" role="menu">
          <button class="menu-item" type="button" @click="emit('rename')">
            <Icon name="pencil" :size="13" /> 重命名
          </button>
          <button class="menu-item" type="button" @click="emit('duplicate')">
            <Icon name="copy" :size="13" /> 复制
          </button>
          <button class="menu-item danger" type="button" @click="emit('delete')">
            <Icon name="trash" :size="13" /> 删除
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ---------- 项目卡片 ---------- */
.proj-card {
  position: relative;
  display: flex;
  gap: 14px;
  padding: 16px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border);
  background: var(--bg-panel);
  box-shadow: var(--shadow);
  cursor: pointer;
  transition:
    border-color var(--dur) var(--ease),
    background var(--dur) var(--ease),
    transform var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.proj-card:hover {
  border-color: rgba(168, 85, 247, 0.5);
  background: rgba(255, 255, 255, 0.03);
  transform: translateY(-2px);
  box-shadow: var(--shadow-lg);
}

.proj-avatar {
  width: 42px;
  height: 42px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius);
  border: 1px solid rgba(255, 255, 255, 0.07);
  font-size: 16px;
  font-weight: 700;
  user-select: none;
}

.proj-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.proj-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.proj-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proj-status {
  flex-shrink: 0;
  font-size: 10.5px;
  font-weight: 600;
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--warning-tint);
  color: var(--warning);
}
.proj-status.active {
  background: var(--success-tint);
  color: var(--success);
}

.proj-desc {
  margin: 0;
  font-size: 12px;
  color: var(--text-2);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.proj-metrics {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-3);
}

.metric {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.metric-sep {
  color: var(--border-strong);
}

/* ---------- 卡片右侧：打开箭头 + 更多菜单 ---------- */
.proj-side {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 8px;
  flex-shrink: 0;
}

.proj-open {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 6px;
  border: none;
  border-radius: 6px;
  background: none;
  font-family: inherit;
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
  cursor: pointer;
  opacity: 0;
  transition:
    opacity var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.proj-card:hover .proj-open {
  opacity: 1;
}
.proj-open:hover {
  background: var(--accent-tint);
}

.proj-more {
  position: relative;
  flex-shrink: 0;
}

.more-menu {
  position: absolute;
  top: 30px;
  right: 0;
  z-index: 10;
  min-width: 132px;
  padding: 4px;
  border-radius: var(--radius);
  border: 1px solid var(--border-strong);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-lg);
  animation: menu-in 120ms var(--ease);
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  height: 30px;
  padding: 0 10px;
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  color: var(--text-1);
  font-size: 12.5px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background var(--dur) var(--ease);
}
.menu-item:hover {
  background: var(--bg-hover);
}
.menu-item.danger {
  color: var(--danger);
}
.menu-item.danger:hover {
  background: var(--danger-tint);
}

@keyframes menu-in {
  from {
    opacity: 0;
    transform: translateY(-3px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>

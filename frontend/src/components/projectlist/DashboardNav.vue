<script setup lang="ts">
/**
 * DashboardNav：仪表板左侧导航。
 *
 * - 仪表板 / API 项目 / 集合 / API 文档 / 设置；
 * - 未实现项点击 Toast 提示「即将提供」；设置项 emit('settings') 打开设置弹窗。
 */
import { useRouter } from 'vue-router'
import Icon from '../ui/Icon.vue'
import { useToast } from '../../composables/useToast'

const router = useRouter()
const toast = useToast()

const emit = defineEmits<{ settings: [] }>()

const NAV_ITEMS = [
  { key: 'dashboard', label: '仪表板', icon: 'gauge' as const, route: '/projects', action: null as null | 'settings' },
  { key: 'projects', label: 'API 项目', icon: 'folder' as const, route: '/projects', action: null },
  { key: 'collections', label: '集合', icon: 'list' as const, route: '', action: null, done: false },
  { key: 'docs', label: 'API 文档', icon: 'file' as const, route: '', action: null, done: false },
  { key: 'settings', label: '设置', icon: 'settings' as const, route: '', action: 'settings' as const },
]

function navActive(item: (typeof NAV_ITEMS)[number]): boolean {
  return router.currentRoute.value.path === '/projects' && item.route === '/projects'
}

function onNav(item: (typeof NAV_ITEMS)[number]): void {
  if (item.action === 'settings') {
    emit('settings')
    return
  }
  if ('done' in item && !item.done) {
    toast.info(`「${item.label}」将在后续版本提供`)
    return
  }
  router.push(item.route)
}
</script>

<template>
  <nav class="dash-nav" aria-label="主导航">
    <button
      v-for="item in NAV_ITEMS"
      :key="item.key"
      class="nav-item"
      :class="{ active: navActive(item), soon: !item.done }"
      type="button"
      @click="onNav(item)"
    >
      <Icon :name="item.icon" :size="15" />
      <span class="nav-label">{{ item.label }}</span>
      <span v-if="!item.done && !item.action" class="nav-soon">即将</span>
    </button>
  </nav>
</template>

<style scoped>
.dash-nav {
  width: 200px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 16px 10px;
  border-right: 1px solid var(--border);
  background: var(--bg-panel);
  overflow-y: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 34px;
  padding: 0 10px;
  border: none;
  border-radius: var(--radius);
  background: none;
  color: var(--text-2);
  font-size: 13px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease);
}
.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text-1);
}
.nav-item:active {
  background: var(--bg-active);
}
.nav-item.active {
  background: var(--accent-tint);
  color: var(--accent);
  font-weight: 600;
}
.nav-item.soon {
  color: var(--text-3);
}

.nav-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav-soon {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--bg-hover);
  color: var(--text-3);
}
</style>

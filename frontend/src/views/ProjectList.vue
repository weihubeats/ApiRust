<script setup lang="ts">
/**
 * ProjectList：仪表板视图（Dashboard）。
 *
 * - 顶部栏：RustFox 品牌 + 设置（无重复搜索框）；
 * - 左侧导航：DashboardNav 组件；
 * - 摘要区：总 API 数 + 最近修改项目 + 快速请求入口；
 * - 工具栏：名称过滤 + 视图切换（网格/列表）+ 排序（最近修改/名称/API数量）+ 新建项目；
 * - 项目卡片 / 弹窗（新建、重命名、删除、快速请求）拆分至 components/projectlist/。
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'
import { ensureSwapMounted } from '../utils/sortable'
import Sortable from 'sortablejs'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import CustomSelect from '../components/ui/CustomSelect.vue'
import Icon from '../components/ui/Icon.vue'
import IconButton from '../components/ui/IconButton.vue'
import SettingsDialog from '../components/SettingsDialog.vue'
import DashboardNav from '../components/projectlist/DashboardNav.vue'
import ProjectCard from '../components/projectlist/ProjectCard.vue'
import ProjectCreateModal from '../components/projectlist/ProjectCreateModal.vue'
import ProjectRenameModal from '../components/projectlist/ProjectRenameModal.vue'
import ProjectDeleteModal from '../components/projectlist/ProjectDeleteModal.vue'
import ScratchRequestModal from '../components/projectlist/ScratchRequestModal.vue'
import { timeAgo } from '../components/projectlist/projectMeta'
import { useWindowDrag } from '../composables/useWindowDrag'
import logo from '../assets/rustfox-logo.png'
import type { Project } from '../types/foxApi'

const api = useFoxApi()
const toast = useToast()
const router = useRouter()

const projects = ref<Project[]>([])
const counts = ref<Record<string, number>>({})
const loading = ref(false)
const loadError = ref<string | null>(null)
const search = ref('')

// ---------- 摘要 ----------
const totalApis = computed(() => Object.values(counts.value).reduce((a, b) => a + b, 0))

const recentProjects = computed(() =>
  [...projects.value].sort((a, b) => b.updated_at.localeCompare(a.updated_at)).slice(0, 2),
)

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase()
  const list = q ? projects.value.filter((p) => p.name.toLowerCase().includes(q)) : [...projects.value]
  switch (sortKey.value) {
    case 'name':
      return list.sort((a, b) => a.name.localeCompare(b.name))
    case 'apis':
      return list.sort((a, b) => (counts.value[b.id] ?? 0) - (counts.value[a.id] ?? 0))
    case 'manual':
      return list
    default:
      return list.sort((a, b) => b.updated_at.localeCompare(a.updated_at))
  }
})

// ---------- 视图切换 / 排序 ----------
const viewMode = ref<'grid' | 'list'>('grid')
const sortKey = ref<'manual' | 'updated' | 'name' | 'apis'>('manual')
const SORT_OPTIONS = [
  { value: 'manual', label: '手动排序' },
  { value: 'updated', label: '最近修改' },
  { value: 'name', label: '名称' },
  { value: 'apis', label: 'API 数量' },
]

// ---------- 拖拽排序 ----------
/** 手动排序 + 无搜索过滤时才允许拖拽（否则顺序无意义）。 */
const dragEnabled = computed(
  () => sortKey.value === 'manual' && !search.value.trim() && projects.value.length > 1,
)

const gridEl = ref<HTMLElement | null>(null)
const sortable = ref<Sortable | null>(null)
const savingOrder = ref(false)

/** 交换式拖拽：A 拖到 B 后面时 B 直接让位到 A 原位，无 ghost 空白占位。 */
ensureSwapMounted()

/** 拖拽结束后的短暂窗口内抑制卡片 click（防止拖完被带进项目）。 */
let suppressClickUntil = 0

/** 按当前 DOM 顺序重排 projects 并持久化。 */
async function onDragEnd(): Promise<void> {
  suppressClickUntil = Date.now() + 400
  const ordered: Project[] = []
  const els = gridEl.value?.querySelectorAll<HTMLElement>('[data-project-id]') ?? []
  for (const el of els) {
    const found = projects.value.find((p) => p.id === el.dataset.projectId)
    if (found) ordered.push(found)
  }
  if (ordered.length !== projects.value.length) return
  projects.value = ordered
  if (savingOrder.value) return
  savingOrder.value = true
  try {
    await api.updateProjectsOrder(ordered.map((p) => p.id))
  } catch (e) {
    toast.error('排序保存失败', { message: e instanceof Error ? e.message : String(e), duration: 4000 })
  } finally {
    savingOrder.value = false
  }
}

/** 拖拽刚结束的 click 不进入项目。 */
function onCardOpen(p: Project): void {
  if (Date.now() < suppressClickUntil) return
  void enter(p)
}

function initSortable(): void {
  if (sortable.value || !gridEl.value) return
  console.log('[dnd] init sortable', gridEl.value)
  sortable.value = Sortable.create(gridEl.value, {
    animation: 200,
    easing: 'cubic-bezier(0.2, 0, 0, 1)',
    swap: true,
    swapClass: 'sortable-swap',
    swapThreshold: 0.75,
    forceFallback: true,
    fallbackClass: 'sortable-drag',
    fallbackOnBody: true,
    fallbackTolerance: 3,
    ghostClass: 'sortable-ghost',
    dragClass: 'sortable-drag',
    chosenClass: 'sortable-chosen',
    disabled: !dragEnabled.value,
    filter: 'button, a, input, select, textarea, [data-no-drag]',
    preventOnFilter: true,
    onChoose: () => console.log('[dnd] choose'),
    onStart: () => console.log('[dnd] start'),
    onEnd: () => {
      console.log('[dnd] end')
      void onDragEnd()
    },
    onMove: (evt) =>
      !evt.related?.closest('button, a, input, select, textarea, [data-no-drag]'),
  })
}

onMounted(initSortable)

/** 列表容器挂载/重建时初始化（v-if 延迟渲染，ref 变化是最可靠信号）。 */
watch(gridEl, (el) => {
  if (el && !sortable.value) initSortable()
})

watch(dragEnabled, (enabled) => {
  sortable.value?.option('disabled', !enabled)
})

onBeforeUnmount(() => {
  sortable.value?.destroy()
  sortable.value = null
})

function isActive(p: Project): boolean {
  return api.activeProject.value?.id === p.id
}

// ---------- 数据 ----------
async function loadCounts(): Promise<void> {
  await Promise.all(
    projects.value.map(async (p) => {
      try {
        const eps = await api.listEndpoints(p.id)
        counts.value[p.id] = eps.length
      } catch {
        counts.value[p.id] = 0
      }
    }),
  )
}

async function load(): Promise<void> {
  if (loading.value) return
  loading.value = true
  loadError.value = null
  try {
    projects.value = await api.getProjects()
    counts.value = {}
    await loadCounts()
  } catch (e) {
    loadError.value = e instanceof Error ? e.message : String(e)
    toast.error('项目列表加载失败', { message: loadError.value, duration: 6000 })
  } finally {
    loading.value = false
  }
}

onMounted(load)

// ---------- 新建 ----------
const showCreate = ref(false)

function onCreated(project: Project): void {
  projects.value.push(project)
  counts.value[project.id] = 0
}

// ---------- 进入工作区 ----------
async function enter(project: Project): Promise<void> {
  try {
    await api.setActiveProject(project.id)
    await api.listEndpoints(project.id)
    toast.info(`已进入项目：${project.name}`)
    router.push('/workspace')
  } catch (e) {
    toast.error('进入项目失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  }
}

// ---------- 快速请求 ----------
const showScratch = ref(false)

// ---------- 卡片菜单：重命名 / 复制 / 删除 ----------
const menuOpenId = ref<string | null>(null)

function toggleMenu(id: string): void {
  menuOpenId.value = menuOpenId.value === id ? null : id
}

function closeMenu(): void {
  menuOpenId.value = null
}

function onDocClick(): void {
  closeMenu()
}

onMounted(() => document.addEventListener('click', onDocClick))
onBeforeUnmount(() => document.removeEventListener('click', onDocClick))

const renaming = ref<Project | null>(null)

function openRename(p: Project): void {
  closeMenu()
  renaming.value = p
}

function onRenamed(saved: Project): void {
  const idx = projects.value.findIndex((p) => p.id === saved.id)
  if (idx !== -1) projects.value[idx] = saved
}

async function duplicate(p: Project): Promise<void> {
  closeMenu()
  const now = new Date().toISOString()
  try {
    const copy = await api.saveProject({
      ...p,
      id: crypto.randomUUID(),
      name: `${p.name} 副本`,
      created_at: now,
      updated_at: now,
    })
    projects.value.push(copy)
    counts.value[copy.id] = 0
    toast.success('项目已复制', { message: copy.name })
  } catch (e) {
    toast.error('复制项目失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  }
}

const deleting = ref<Project | null>(null)

function openDelete(p: Project): void {
  closeMenu()
  deleting.value = p
}

async function onDeleted(id: string): Promise<void> {
  projects.value = projects.value.filter((p) => p.id !== id)
  delete counts.value[id]
  if (api.activeProject.value?.id === id) {
    await api.setActiveProject(null).catch(() => undefined)
  }
}

const showSettings = ref(false)

/** 顶栏拖拽窗口（与工作区顶栏共用 useWindowDrag）。 */
const topBarEl = ref<HTMLElement | null>(null)
useWindowDrag(topBarEl)
</script>

<template>
  <div class="dash">
    <header ref="topBarEl" class="dash-top">
      <button class="top-brand" type="button" title="回到项目首页" @click="router.push('/projects')">
        <span class="top-logo" aria-hidden="true">
          <img :src="logo" alt="" width="18" height="18" />
        </span>
        <span class="top-title">RustFox</span>
        <span class="top-tag">API 调试工具</span>
      </button>
      <div class="top-right">
        <IconButton name="settings" :size="15" title="设置" @click="showSettings = true" />
      </div>
    </header>

    <div class="dash-body">
      <DashboardNav @settings="showSettings = true" />

      <main class="dash-main">
        <div v-if="loadError" class="rf-inline-error" role="alert">
          <span class="rf-inline-error-text">加载失败：{{ loadError }}</span>
          <button class="rf-btn rf-btn-sm" type="button" :disabled="loading" @click="load">
            {{ loading ? '重试中…' : '重试' }}
          </button>
        </div>

        <template v-else>
          <section class="summary-grid">
            <div class="stat-card">
              <span class="stat-icon"><Icon name="gauge" :size="16" /></span>
              <div class="stat-body">
                <span class="stat-label">总 API 数</span>
                <span class="stat-value num">{{ totalApis }}</span>
                <span class="stat-sub">分布在 {{ projects.length }} 个项目</span>
              </div>
            </div>
            <div class="stat-card">
              <span class="stat-icon"><Icon name="clock" :size="16" /></span>
              <div class="stat-body">
                <span class="stat-label">最近活动</span>
                <div v-if="recentProjects.length" class="stat-recent">
                  <button
                    v-for="p in recentProjects"
                    :key="p.id"
                    class="recent-item"
                    type="button"
                    @click="enter(p)"
                  >
                    <span class="recent-name">{{ p.name }}</span>
                    <span class="recent-time">{{ timeAgo(p.updated_at) }}</span>
                  </button>
                </div>
                <span v-else class="stat-sub">暂无项目</span>
              </div>
            </div>
            <div class="stat-card">
              <span class="stat-icon"><Icon name="send" :size="16" /></span>
              <div class="stat-body">
                <span class="stat-label">快速开始</span>
                <p class="stat-sub">不保存项目，直接发送临时请求</p>
                <button class="quick-btn" type="button" @click="showScratch = true">
                  <Icon name="zap" :size="13" /> 快速请求
                </button>
              </div>
            </div>
          </section>

          <section class="toolbar">
            <div class="toolbar-filter">
              <Icon name="search" :size="14" />
              <input
                v-model="search"
                class="toolbar-filter-input"
                placeholder="按名称过滤项目…"
                spellcheck="false"
              />
            </div>
            <div class="toolbar-view" role="group" aria-label="视图切换">
              <button
                type="button"
                class="view-btn"
                :class="{ on: viewMode === 'grid' }"
                title="网格视图"
                @click="viewMode = 'grid'"
              >
                <Icon name="layout-grid" :size="14" />
              </button>
              <button
                type="button"
                class="view-btn"
                :class="{ on: viewMode === 'list' }"
                title="列表视图"
                @click="viewMode = 'list'"
              >
                <Icon name="list" :size="14" />
              </button>
            </div>
            <CustomSelect
              :model-value="sortKey"
              :options="SORT_OPTIONS"
              size="sm"
              class="toolbar-sort"
              @update:model-value="sortKey = String($event) as 'updated' | 'name' | 'apis'"
            />
            <button class="btn-new" type="button" @click="showCreate = true">
              <Icon name="plus" :size="15" /> 新建 API 项目
            </button>
          </section>

          <div
            v-if="filtered.length"
            ref="gridEl"
            class="card-grid"
            :class="{ list: viewMode === 'list' }"
          >
            <ProjectCard
              v-for="p in filtered"
              :key="p.id"
              :project="p"
              :count="counts[p.id] ?? 0"
              :active="isActive(p)"
              :menu-open="menuOpenId === p.id"
              :draggable="dragEnabled"
              @open="onCardOpen(p)"
              @toggle-menu="toggleMenu(p.id)"
              @rename="openRename(p)"
              @duplicate="duplicate(p)"
              @delete="openDelete(p)"
            />
          </div>

          <div v-else-if="loading" class="dash-empty">
            <p class="rf-hint">加载中…</p>
          </div>

          <div v-else class="dash-empty">
            <span class="empty-icon"><Icon name="folder" :size="30" /></span>
            <p class="empty-title">{{ search ? '没有匹配的项目' : '还没有项目' }}</p>
            <p class="empty-hint">
              {{ search ? '换个关键词试试，或创建一个新项目。' : '创建你的第一个 API 项目，开始设计、调试与 Mock。' }}
            </p>
            <button class="rf-btn rf-btn-primary" type="button" @click="showCreate = true">
              <Icon name="plus" :size="14" /> 新建 API 项目
            </button>
          </div>
        </template>
      </main>
    </div>

    <ProjectCreateModal v-model:open="showCreate" @created="onCreated" />

    <ProjectRenameModal :project="renaming" @close="renaming = null" @saved="onRenamed" />

    <ProjectDeleteModal :project="deleting" @close="deleting = null" @deleted="onDeleted" />

    <ScratchRequestModal v-model:open="showScratch" />

    <SettingsDialog v-if="showSettings" @close="showSettings = false" />
  </div>
</template>

<style scoped>
.dash {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg-app);
}

/* ---------- 顶部栏 ---------- */
.dash-top {
  height: 56px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 16px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
  cursor: grab;
  user-select: none;
}

.top-brand {
  display: flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
  padding: 4px 8px;
  margin-left: -8px;
  border: none;
  background: none;
  border-radius: var(--radius);
  cursor: pointer;
  transition: background var(--dur) var(--ease);
}
.top-brand:hover {
  background: var(--bg-hover);
}
.top-brand:active {
  background: var(--bg-active);
}

.top-logo {
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  border-radius: 8px;
  background: linear-gradient(135deg, var(--accent) 0%, var(--put) 100%);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.25);
}

.top-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-1);
  letter-spacing: 0.01em;
}

.top-tag {
  font-size: 11px;
  color: var(--text-3);
  padding-left: 9px;
  border-left: 1px solid var(--border-strong);
  white-space: nowrap;
}

.top-right {
  margin-left: auto;
  display: flex;
  align-items: center;
}

/* ---------- 主体：导航 + 内容 ---------- */
.dash-body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.dash-main {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 24px 28px 32px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

/* ---------- 摘要卡片 ---------- */
.summary-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 16px;
}

.stat-card {
  display: flex;
  gap: 14px;
  padding: 18px 20px;
  border-radius: var(--radius-lg);
  border: 1px solid var(--border);
  background: var(--bg-panel);
  box-shadow: var(--shadow);
  transition:
    border-color var(--dur) var(--ease),
    transform var(--dur) var(--ease);
}
.stat-card:hover {
  border-color: var(--border-strong);
  transform: translateY(-1px);
}

.stat-icon {
  width: 38px;
  height: 38px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius);
  background: var(--accent-tint);
  color: var(--accent);
}

.stat-body {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.stat-label {
  font-size: 12px;
  color: var(--text-2);
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  line-height: 1.2;
  color: var(--text-1);
  font-variant-numeric: tabular-nums;
}

.stat-sub {
  font-size: 11.5px;
  color: var(--text-3);
}

.stat-recent {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 4px;
}

.recent-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  border: none;
  background: none;
  padding: 4px 6px;
  border-radius: var(--radius);
  cursor: pointer;
  font-family: inherit;
  text-align: left;
  transition: background var(--dur) var(--ease);
}
.recent-item:hover {
  background: var(--bg-hover);
}

.recent-name {
  font-size: 12.5px;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.recent-time {
  font-size: 11px;
  color: var(--text-3);
  flex-shrink: 0;
}

.quick-btn {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  margin-top: 6px;
  height: 28px;
  padding: 0 12px;
  border: 1px solid var(--accent-tint);
  border-radius: var(--radius);
  background: var(--accent-tint);
  color: var(--accent);
  font-size: 12.5px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition:
    background var(--dur) var(--ease),
    border-color var(--dur) var(--ease),
    transform var(--dur) var(--ease);
}
.quick-btn:hover {
  background: rgba(168, 85, 247, 0.22);
  border-color: var(--accent);
}
.quick-btn:active {
  transform: translateY(1px);
}

/* ---------- 工具栏：过滤 + 视图切换 + 排序 + 新建 ---------- */
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toolbar-filter {
  flex: 1;
  max-width: 320px;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  padding: 0 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  color: var(--text-3);
  transition:
    border-color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.toolbar-filter:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint);
}

.toolbar-filter-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: none;
  color: var(--text-1);
  font-family: inherit;
  font-size: 12.5px;
}
.toolbar-filter-input::placeholder {
  color: var(--text-3);
}

.toolbar-view {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 2px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-card);
}

.view-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: var(--radius-sm);
  background: none;
  color: var(--text-3);
  cursor: pointer;
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease);
}
.view-btn:hover {
  color: var(--text-1);
  background: var(--bg-hover);
}
.view-btn.on {
  background: var(--accent);
  color: #fff;
}
.view-btn.on:hover {
  background: var(--accent-hover);
}

.toolbar-sort {
  width: 130px;
}

.btn-new {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  height: 34px;
  padding: 0 16px;
  border: none;
  border-radius: var(--radius-lg);
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  box-shadow: 0 4px 14px var(--accent-tint);
  transition:
    background var(--dur) var(--ease),
    transform var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.btn-new:hover {
  background: var(--accent-hover);
  box-shadow: 0 6px 18px var(--accent-tint);
}
.btn-new:active {
  transform: translateY(1px);
}

/* ---------- 项目卡片网格（卡片自身样式见 ProjectCard） ---------- */
.card-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 16px;
}
@media (min-width: 768px) {
  .card-grid {
    grid-template-columns: repeat(2, 1fr);
  }
}
@media (min-width: 1024px) {
  .card-grid {
    grid-template-columns: repeat(3, 1fr);
  }
}
@media (min-width: 1280px) {
  .card-grid {
    grid-template-columns: repeat(4, 1fr);
  }
}

/* 列表视图：单列横向行（穿透到子组件的 .proj-card 等） */
.card-grid.list {
  grid-template-columns: 1fr;
}
.card-grid.list :deep(.proj-card) {
  align-items: center;
  padding: 12px 16px;
}
.card-grid.list :deep(.proj-main) {
  flex-direction: row;
  align-items: center;
  gap: 16px;
}
.card-grid.list :deep(.proj-title-row) {
  flex-shrink: 0;
  min-width: 180px;
  max-width: 260px;
}
.card-grid.list :deep(.proj-desc) {
  flex: 1;
  max-width: none;
}
.card-grid.list :deep(.proj-metrics) {
  flex-shrink: 0;
  margin-left: auto;
}

/* ---------- 拖拽排序 ---------- */
.card-grid :deep(.proj-card) {
  cursor: grab;
}
.card-grid :deep(.proj-card:active) {
  cursor: grabbing;
}

/* 拖拽中元素禁用自身 transform 过渡：SortableJS 独占移动控制，
   否则卡片 hover 缓动与拖拽动画互相干扰（拖动生硬、掉帧）。 */
:global(.sortable-ghost),
:global(.sortable-drag),
:global(.sortable-chosen) {
  transition: none !important;
  will-change: transform;
}

/* swap：交换目标卡片高亮反馈 */
:global(.sortable-swap) {
  border: 1px solid color-mix(in srgb, var(--accent) 50%, transparent) !important;
  background: color-mix(in srgb, var(--accent) 8%, transparent) !important;
  transition: none !important;
}

/* ghost：原位置镂空占位（ghost 即原卡片元素，内部全部隐藏 + 极淡底色） */
:global(.sortable-ghost) {
  background-color: rgba(168, 85, 247, 0.05) !important;
  border: 2px dashed rgba(168, 85, 247, 0.4) !important;
  border-radius: 0.75rem !important;
  box-shadow: none !important;
  opacity: 1;
  cursor: grabbing;
}
:global(.sortable-ghost *) {
  opacity: 0 !important;
  visibility: hidden !important;
}

/* chosen：mousedown 即给原元素反馈（拖起瞬间轻微放大） */
:global(.sortable-chosen) {
  transform: scale(1.03);
  cursor: grabbing;
}

/* drag：被拖主体（fallbackClass 克隆挂 body，悬浮感最强） */
:global(.sortable-drag) {
  z-index: 999;
  border-radius: var(--radius-lg) !important;
  background: rgba(38, 38, 38, 0.95) !important;
  -webkit-backdrop-filter: blur(8px);
  backdrop-filter: blur(8px);
  border: 1px solid #a855f7 !important;
  box-shadow:
    0 20px 50px rgba(0, 0, 0, 0.8),
    0 0 40px rgba(59, 7, 100, 0.4) !important;
  transform: scale(1.03) rotate(1.5deg);
  cursor: grabbing;
}

/* flip：其余卡片平滑挤开（animation 200ms 已生效，此为兜底） */
.card-grid :deep(.flip-list-move) {
  transition: transform 0.2s cubic-bezier(0.2, 0, 0, 1);
}

/* ---------- 空状态 ---------- */
.dash-empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 48px 0;
  text-align: center;
}

.empty-icon {
  color: var(--text-3);
  margin-bottom: 6px;
}

.empty-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-1);
}

.empty-hint {
  margin: 0 0 10px;
  font-size: 12.5px;
  color: var(--text-3);
  max-width: 380px;
}
</style>

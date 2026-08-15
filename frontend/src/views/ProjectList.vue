<script setup lang="ts">
/**
 * ProjectList：仪表板视图（Dashboard）。
 *
 * - 顶部栏：RustFox 品牌 + 设置（无重复搜索框）；
 * - 左侧导航：仪表板 / API 项目 / 集合 / API 文档 / 设置（未实现项点击 Toast 提示）；
 * - 摘要区：总 API 数 + 最近修改项目 + 快速请求入口；
 * - 工具栏：名称过滤 + 视图切换（网格/列表）+ 排序（最近修改/名称/API数量）+ 新建项目；
 * - 项目卡片：彩色渐变头像 / 状态标签 / 指标 / 更多菜单 / hover 打开箭头。
 */
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import { formatBytes, formatDuration } from '../utils/format'
import CustomSelect from '../components/ui/CustomSelect.vue'
import Icon from '../components/ui/Icon.vue'
import IconButton from '../components/ui/IconButton.vue'
import Modal from '../components/ui/Modal.vue'
import SettingsDialog from '../components/SettingsDialog.vue'
import type { BodySpec, ExecuteResponse, HttpMethod, Project } from '../types/foxApi'

const api = useFoxApi()
const toast = useToast()
const router = useRouter()

const NAME_MAX = 50
const DESC_MAX = 200

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
    default:
      return list.sort((a, b) => b.updated_at.localeCompare(a.updated_at))
  }
})

// ---------- 视图切换 / 排序 ----------
const viewMode = ref<'grid' | 'list'>('grid')
const sortKey = ref<'updated' | 'name' | 'apis'>('updated')
const SORT_OPTIONS = [
  { value: 'updated', label: '最近修改' },
  { value: 'name', label: '名称' },
  { value: 'apis', label: 'API 数量' },
]

// ---------- 工具 ----------
const PALETTE = [
  { bg: 'rgba(124, 105, 245, 0.16)', color: '#a78bfa' },
  { bg: 'rgba(34, 197, 94, 0.14)', color: '#34d399' },
  { bg: 'rgba(59, 130, 246, 0.14)', color: '#60a5fa' },
  { bg: 'rgba(245, 158, 11, 0.14)', color: '#fbbf24' },
  { bg: 'rgba(236, 72, 153, 0.14)', color: '#f472b6' },
  { bg: 'rgba(6, 182, 212, 0.14)', color: '#22d3ee' },
]

function avatarStyle(name: string): { background: string; color: string } {
  let h = 0
  for (let i = 0; i < name.length; i++) h = (h * 31 + name.charCodeAt(i)) >>> 0
  const c1 = PALETTE[h % PALETTE.length]
  const c2 = PALETTE[(h * 2654435761) % PALETTE.length]
  return {
    background: `linear-gradient(135deg, ${c1.color}2e 0%, ${c2.color}2e 100%)`,
    color: c1.color,
  }
}

function initials(name: string): string {
  const parts = name.trim().split(/\s+/).filter(Boolean)
  if (parts.length >= 2) return (parts[0][0] + parts[1][0]).toUpperCase()
  return name.trim().slice(0, 1).toUpperCase() || '?'
}

function timeAgo(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  const min = Math.floor(diff / 60000)
  if (min < 1) return '刚刚'
  if (min < 60) return `${min} 分钟前`
  const hours = Math.floor(min / 60)
  if (hours < 24) return `${hours} 小时前`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days} 天前`
  return iso.slice(0, 10)
}

function statusOf(p: Project): { label: string; active: boolean } {
  return api.activeProject.value?.id === p.id ? { label: 'Active', active: true } : { label: 'Draft', active: false }
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
const newName = ref('')
const newDesc = ref('')
const createError = ref<string | null>(null)

function openCreate(): void {
  newName.value = ''
  newDesc.value = ''
  createError.value = null
  showCreate.value = true
}

async function confirmCreate(): Promise<void> {
  const name = newName.value.trim()
  if (!name) {
    createError.value = '项目名称不能为空'
    return
  }
  if (name.length > NAME_MAX) {
    createError.value = `项目名称不能超过 ${NAME_MAX} 个字符`
    return
  }
  const now = new Date().toISOString()
  try {
    const project = await api.saveProject({
      id: crypto.randomUUID(),
      name,
      description: newDesc.value.trim(),
      variables: {},
      created_at: now,
      updated_at: now,
    })
    projects.value.push(project)
    counts.value[project.id] = 0
    showCreate.value = false
    toast.success('项目创建成功', { message: name })
  } catch (e) {
    toast.error('创建项目失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  }
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

// ---------- 快速请求（暂存区，不落库） ----------
const showScratch = ref(false)
const scratchMethod = ref<HttpMethod>('GET')
const scratchUrl = ref('')
const scratchBody = ref('')
const scratchSending = ref(false)
const scratchRes = ref<ExecuteResponse | null>(null)
const SCRATCH_METHODS: Array<{ value: HttpMethod; label: string }> = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'DELETE', label: 'DELETE' },
]

function openScratch(): void {
  scratchUrl.value = ''
  scratchBody.value = ''
  scratchRes.value = null
  showScratch.value = true
}

async function sendScratch(): Promise<void> {
  const url = scratchUrl.value.trim()
  if (!url) {
    toast.warning('请输入请求地址')
    return
  }
  scratchSending.value = true
  scratchRes.value = null
  try {
    let body: BodySpec = { mode: 'none' }
    if (scratchMethod.value !== 'GET' && scratchBody.value.trim()) {
      const raw = scratchBody.value.trim()
      try {
        JSON.parse(raw)
        body = { mode: 'json', raw }
      } catch {
        body = { mode: 'text', raw }
      }
    }
    scratchRes.value = await api.executeRequest({
      url,
      method: scratchMethod.value,
      spec: {
        params: [],
        headers: [],
        path_variables: [],
        auth: { type: 'none' },
        body,
        timeout_ms: 30000,
        follow_redirects: true,
        tests: null,
      },
      environment_id: null,
    })
  } catch (e) {
    toast.error('请求失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  } finally {
    scratchSending.value = false
  }
}

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
const renameName = ref('')
const renameError = ref<string | null>(null)

function openRename(p: Project): void {
  closeMenu()
  renaming.value = p
  renameName.value = p.name
  renameError.value = null
}

async function confirmRename(): Promise<void> {
  const name = renameName.value.trim()
  if (!name) {
    renameError.value = '项目名称不能为空'
    return
  }
  if (name.length > NAME_MAX) {
    renameError.value = `项目名称不能超过 ${NAME_MAX} 个字符`
    return
  }
  if (!renaming.value) return
  try {
    const saved = await api.saveProject({ ...renaming.value, name, updated_at: new Date().toISOString() })
    const idx = projects.value.findIndex((p) => p.id === saved.id)
    if (idx !== -1) projects.value[idx] = saved
    renaming.value = null
    toast.success('项目已重命名')
  } catch (e) {
    toast.error('重命名失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  }
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

async function confirmDelete(): Promise<void> {
  if (!deleting.value) return
  const target = deleting.value
  try {
    await api.deleteProject(target.id)
    projects.value = projects.value.filter((p) => p.id !== target.id)
    delete counts.value[target.id]
    if (api.activeProject.value?.id === target.id) {
      await api.setActiveProject(null).catch(() => undefined)
    }
    deleting.value = null
    toast.success('项目已删除', { message: target.name })
  } catch (e) {
    toast.error('删除失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  }
}

// ---------- 导航 ----------
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
    showSettings.value = true
    return
  }
  if ('done' in item && !item.done) {
    toast.info(`「${item.label}」将在后续版本提供`)
    return
  }
  router.push(item.route)
}

const showSettings = ref(false)
</script>

<template>
  <div class="dash">
    <header class="dash-top">
      <button class="top-brand" type="button" title="回到项目首页" @click="router.push('/projects')">
        <span class="top-logo" aria-hidden="true">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
            <path d="M13.2 2 4.4 13.6h6.2L9.1 22l8.9-11.6h-6.3L13.2 2z" fill="currentColor" />
          </svg>
        </span>
        <span class="top-title">RustFox</span>
        <span class="top-tag">API 调试工具</span>
      </button>
      <div class="top-right">
        <IconButton name="settings" :size="15" title="设置" @click="showSettings = true" />
      </div>
    </header>

    <div class="dash-body">
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
                <button class="quick-btn" type="button" @click="openScratch">
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
            <button class="btn-new" type="button" @click="openCreate">
              <Icon name="plus" :size="15" /> 新建 API 项目
            </button>
          </section>

          <div v-if="filtered.length" class="card-grid" :class="{ list: viewMode === 'list' }">
            <div v-for="p in filtered" :key="p.id" class="proj-card" @click="enter(p)">
              <span class="proj-avatar" :style="avatarStyle(p.name)">{{ initials(p.name) }}</span>
              <div class="proj-main">
                <div class="proj-title-row">
                  <span class="proj-title" :title="p.name">{{ p.name }}</span>
                  <span class="proj-status" :class="{ active: statusOf(p).active }">
                    {{ statusOf(p).label }}
                  </span>
                </div>
                <p class="proj-desc">{{ p.description || '暂无描述' }}</p>
                <div class="proj-metrics">
                  <span class="metric"><Icon name="plug" :size="12" />{{ counts[p.id] ?? 0 }} APIs</span>
                  <span class="metric-sep">·</span>
                  <span class="metric"><Icon name="clock" :size="12" />{{ timeAgo(p.updated_at) }}</span>
                </div>
              </div>
              <div class="proj-side">
                <span class="proj-open" title="打开项目">
                  <Icon name="arrow-up-right" :size="13" /> Open
                </span>
                <div class="proj-more" @click.stop>
                  <IconButton name="more-horizontal" :size="16" title="更多操作" @click="toggleMenu(p.id)" />
                  <div v-if="menuOpenId === p.id" class="more-menu" role="menu">
                    <button class="menu-item" type="button" @click="openRename(p)">
                      <Icon name="pencil" :size="13" /> 重命名
                    </button>
                    <button class="menu-item" type="button" @click="duplicate(p)">
                      <Icon name="copy" :size="13" /> 复制
                    </button>
                    <button class="menu-item danger" type="button" @click="openDelete(p)">
                      <Icon name="trash" :size="13" /> 删除
                    </button>
                  </div>
                </div>
              </div>
            </div>
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
            <button class="rf-btn rf-btn-primary" type="button" @click="openCreate">
              <Icon name="plus" :size="14" /> 新建 API 项目
            </button>
          </div>
        </template>
      </main>
    </div>

    <Modal v-model:open="showCreate" title="新建 API 项目" width="420px" @close="showCreate = false">
      <div class="form-field">
        <label class="form-label" for="new-project-name">项目名称</label>
        <input
          id="new-project-name"
          v-model="newName"
          class="rf-input"
          :class="{ 'rf-input-error': createError }"
          placeholder="例如：电子商务后端 API"
          maxlength="60"
          spellcheck="false"
          @input="createError = null"
          @keyup.enter="confirmCreate"
        />
        <p v-if="createError" class="rf-field-error" role="alert">{{ createError }}</p>
      </div>
      <div class="form-field">
        <label class="form-label" for="new-project-desc">描述（可选）</label>
        <textarea
          id="new-project-desc"
          v-model="newDesc"
          class="rf-textarea"
          :maxlength="DESC_MAX"
          placeholder="项目用途与说明…"
          rows="3"
        ></textarea>
      </div>
      <template #footer>
        <button class="rf-btn" type="button" @click="showCreate = false">取消</button>
        <button class="rf-btn rf-btn-primary" type="button" :disabled="api.pending.value" @click="confirmCreate">
          创建
        </button>
      </template>
    </Modal>

    <Modal :open="renaming !== null" title="重命名项目" width="420px" @close="renaming = null">
      <div class="form-field">
        <label class="form-label" for="rename-project">项目名称</label>
        <input
          id="rename-project"
          v-model="renameName"
          class="rf-input"
          :class="{ 'rf-input-error': renameError }"
          placeholder="项目名称"
          maxlength="60"
          spellcheck="false"
          @input="renameError = null"
          @keyup.enter="confirmRename"
        />
        <p v-if="renameError" class="rf-field-error" role="alert">{{ renameError }}</p>
      </div>
      <template #footer>
        <button class="rf-btn" type="button" @click="renaming = null">取消</button>
        <button class="rf-btn rf-btn-primary" type="button" :disabled="api.pending.value" @click="confirmRename">
          保存
        </button>
      </template>
    </Modal>

    <Modal :open="deleting !== null" title="删除项目" width="380px" @close="deleting = null">
      <p class="confirm-hint">
        确认删除「{{ deleting?.name }}」？项目下的全部接口、环境与示例将一并删除，此操作不可恢复。
      </p>
      <template #footer>
        <button class="rf-btn" type="button" @click="deleting = null">取消</button>
        <button class="rf-btn rf-btn-danger-solid" type="button" :disabled="api.pending.value" @click="confirmDelete">
          删除
        </button>
      </template>
    </Modal>

    <Modal v-model:open="showScratch" title="快速请求（暂存区）" width="580px" @close="showScratch = false">
      <div class="scratch-row">
        <CustomSelect
          v-model="scratchMethod"
          :options="SCRATCH_METHODS"
          size="sm"
          class="scratch-method"
        />
        <input
          v-model="scratchUrl"
          class="rf-input scratch-url"
          placeholder="https://api.example.com/posts"
          spellcheck="false"
          @keyup.enter="sendScratch"
        />
        <button
          class="rf-btn rf-btn-primary"
          type="button"
          :disabled="scratchSending"
          @click="sendScratch"
        >
          <Icon name="send" :size="13" /> {{ scratchSending ? '发送中…' : '发送' }}
        </button>
      </div>
      <textarea
        v-if="scratchMethod !== 'GET'"
        v-model="scratchBody"
        class="rf-input scratch-ta"
        placeholder="请求体（JSON 或纯文本，可选）"
        spellcheck="false"
      ></textarea>
      <div v-if="scratchRes" class="scratch-res">
        <div class="scratch-res-top">
          <span class="sr-status" :class="{ ok: scratchRes.status < 400, err: scratchRes.status >= 400 }">
            {{ scratchRes.status }}
          </span>
          <span class="sr-meta"><Icon name="clock" :size="11" /> {{ formatDuration(scratchRes.duration_ms) }}</span>
          <span class="sr-meta"><Icon name="download" :size="11" /> {{ formatBytes(scratchRes.size_bytes) }}</span>
        </div>
        <pre class="sr-body">{{ scratchRes.body }}</pre>
      </div>
      <p v-else class="rf-hint scratch-hint">响应将显示在这里（暂存请求不写入历史）。</p>
    </Modal>

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

.top-avatar {
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 50%;
  font-size: 13px;
  font-weight: 700;
  color: #fff;
  background: linear-gradient(135deg, var(--accent) 0%, var(--info) 100%);
  user-select: none;
}

/* ---------- 主体：导航 + 内容 ---------- */
.dash-body {
  flex: 1;
  min-height: 0;
  display: flex;
}

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

/* ---------- 项目卡片 ---------- */
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

/* 列表视图：单列横向行 */
.card-grid.list {
  grid-template-columns: 1fr;
}
.card-grid.list .proj-card {
  align-items: center;
  padding: 12px 16px;
}
.card-grid.list .proj-main {
  flex-direction: row;
  align-items: center;
  gap: 16px;
}
.card-grid.list .proj-title-row {
  flex-shrink: 0;
  min-width: 180px;
  max-width: 260px;
}
.card-grid.list .proj-desc {
  flex: 1;
  max-width: none;
}
.card-grid.list .proj-metrics {
  flex-shrink: 0;
  margin-left: auto;
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

/* ---------- 快速请求暂存区 ---------- */
.scratch-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.scratch-method {
  width: 96px;
  flex-shrink: 0;
}

.scratch-url {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 12.5px;
}

.scratch-ta {
  width: 100%;
  min-height: 90px;
  margin-top: 8px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  resize: vertical;
}

.scratch-res {
  margin-top: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  overflow: hidden;
}

.scratch-res-top {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 7px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-card);
}

.sr-status {
  padding: 1px 9px;
  border-radius: 999px;
  font-family: var(--font-mono);
  font-size: 11.5px;
  font-weight: 700;
}
.sr-status.ok {
  background: var(--success-tint);
  color: var(--success);
}
.sr-status.err {
  background: var(--danger-tint);
  color: var(--danger);
}

.sr-meta {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-3);
}

.sr-body {
  margin: 0;
  max-height: 240px;
  overflow: auto;
  padding: 10px 12px;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-1);
  white-space: pre-wrap;
  word-break: break-all;
}

.scratch-hint {
  margin-top: 10px;
}

/* ---------- 表单 ---------- */
.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.form-label {
  font-size: 12px;
  color: var(--text-2);
}

.confirm-hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-2);
  line-height: 1.6;
  word-break: break-all;
}

.rf-input-error {
  border-color: var(--danger) !important;
}
.rf-input-error:focus {
  border-color: var(--danger) !important;
  box-shadow: 0 0 0 2px var(--danger-tint) !important;
}
</style>
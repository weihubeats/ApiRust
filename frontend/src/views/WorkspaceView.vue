<script setup lang="ts">
/**
 * WorkspaceView：工作区主视图。
 * 左侧接口树（文件夹 + 接口，含 CRUD），右侧标签页 + 编辑器 + 响应区。
 * 树操作全部走 workspace store（Pinia），点击接口打开草稿标签页。
 */
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '../stores/workspace'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import Brand from '../components/Brand.vue'
import EndpointTree from '../components/EndpointTree.vue'
import EnvironmentBar from '../components/EnvironmentBar.vue'
import HistoryPanel from '../components/HistoryPanel.vue'
import Icon from '../components/ui/Icon.vue'
import IconButton from '../components/ui/IconButton.vue'
import Menu, { type MenuItem } from '../components/ui/Menu.vue'
import Modal from '../components/ui/Modal.vue'
import SettingsDialog from '../components/SettingsDialog.vue'
import TabBar from '../components/TabBar.vue'
import Tabs, { type TabItem } from '../components/ui/Tabs.vue'
import EndpointEditor from '../components/EndpointEditor.vue'
import CurlImportDialog from '../components/CurlImportDialog.vue'
import ImportDialog from '../components/ImportDialog.vue'
import MockRuleDialog from '../components/MockRuleDialog.vue'

const store = useWorkspaceStore()
const router = useRouter()
const api = useFoxApi()
const toast = useToast()

const loading = ref(false)
const showCurlImport = ref(false)
const curlFolderId = ref<string | null>(null)
const showDocImport = ref(false)
const showMockRules = ref(false)
const showSettings = ref(false)
const apiSearch = ref('')

// ---------- 侧栏页签：接口目录 / 请求历史 ----------
type SidebarTab = 'collections' | 'history'
const sidebarTab = ref<SidebarTab>('collections')
const sidebarTabs = computed<TabItem[]>(() => [
  { key: 'collections', label: '接口目录' },
  { key: 'history', label: '请求历史', count: store.histories.length || undefined },
])

// ---------- Mock 服务 ----------
const mockAddress = ref<string | null>(null)
const mockBusy = ref(false)

async function refreshMockStatus(): Promise<void> {
  try {
    mockAddress.value = (await api.mockStatus()) ?? null
  } catch {
    mockAddress.value = null
  }
}

async function toggleMock(): Promise<void> {
  if (mockBusy.value) return
  mockBusy.value = true
  try {
    if (mockAddress.value) {
      await api.mockStop()
      mockAddress.value = null
      toast.success('Mock 服务已停止')
    } else {
      mockAddress.value = await api.mockStart()
      toast.success(`Mock 服务已启动：${mockAddress.value}`)
    }
  } catch (err) {
    toast.error('Mock 服务操作失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    mockBusy.value = false
  }
}

async function load(): Promise<void> {
  loading.value = true
  try {
    if (!store.project) {
      const p = await store.init()
      if (!p) {
        router.replace('/projects')
        return
      }
    } else {
      await store.refresh()
    }
  } catch {
    // loadError 已在 store 内写入，界面展示重试
  } finally {
    loading.value = false
  }
}

async function exportOpenapi(): Promise<void> {
  if (!store.project) return
  try {
    const text = await api.exportOpenapi(store.project.id)
    const blob = new Blob([text], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${store.project.name}-openapi.json`
    a.click()
    URL.revokeObjectURL(url)
    toast.success('已导出 OpenAPI 3.0 JSON')
  } catch (err) {
    toast.error('导出失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

function openCurlImport(folderId: string | null): void {
  curlFolderId.value = folderId
  showCurlImport.value = true
}

// ---------- 顶栏下拉菜单（文档 / Mock） ----------
const docsBtn = ref<HTMLButtonElement | null>(null)
const mockBtn = ref<HTMLButtonElement | null>(null)
const menuRef = ref<InstanceType<typeof Menu> | null>(null)

// ---------- 项目切换器（侧栏头部左侧） ----------
const projectBtn = ref<HTMLButtonElement | null>(null)
const treeRef = ref<InstanceType<typeof EndpointTree> | null>(null)

async function openProjectMenu(): Promise<void> {
  if (!projectBtn.value) return
  let items: MenuItem[] = []
  try {
    items = (await api.getProjects()).map((p) => ({
      key: `switch-project:${p.id}`,
      label: p.name,
      icon: p.id === store.project?.id ? 'check' : ('folder' as const),
    }))
  } catch {
    toast.error('项目列表加载失败')
  }
  items.push({ key: 'new-project', label: '新建项目', icon: 'plus', dividerBefore: true })
  menuRef.value?.openAt(projectBtn.value, items, 'left')
}

async function switchProject(projectId: string): Promise<void> {
  try {
    await store.switchProject(projectId)
    toast.success(`已切换到项目：${store.project?.name ?? ''}`)
  } catch (err) {
    toast.error('切换项目失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

// ---------- 项目创建 / 重命名 / 删除 ----------
const showCreateProject = ref(false)
const newProjectName = ref('')
const newProjectDesc = ref('')
const projectFormError = ref<string | null>(null)

function openCreateProject(): void {
  newProjectName.value = ''
  newProjectDesc.value = ''
  projectFormError.value = null
  showCreateProject.value = true
}

async function confirmCreateProject(): Promise<void> {
  const name = newProjectName.value.trim()
  if (!name) {
    projectFormError.value = '项目名称不能为空'
    return
  }
  const now = new Date().toISOString()
  try {
    const p = await api.saveProject({
      id: crypto.randomUUID(),
      name,
      description: newProjectDesc.value.trim(),
      variables: {},
      created_at: now,
      updated_at: now,
    })
    showCreateProject.value = false
    await switchProject(p.id)
    toast.success('项目创建成功', { message: name })
  } catch (err) {
    toast.error('创建项目失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

const renamingProject = ref(false)
const renameProjectName = ref('')

function openRenameProject(): void {
  renameProjectName.value = store.project?.name ?? ''
  projectFormError.value = null
  renamingProject.value = true
}

async function confirmRenameProject(): Promise<void> {
  const name = renameProjectName.value.trim()
  if (!name) {
    projectFormError.value = '项目名称不能为空'
    return
  }
  const current = store.project
  if (!current) return
  try {
    const saved = await api.saveProject({ ...current, name, updated_at: new Date().toISOString() })
    store.project = saved
    renamingProject.value = false
    toast.success('项目已重命名')
  } catch (err) {
    toast.error('重命名失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

async function confirmDeleteProject(): Promise<void> {
  const target = store.project
  if (!target) return
  try {
    await api.deleteProject(target.id)
    await api.setActiveProject(null).catch(() => undefined)
    toast.success('项目已删除', { message: target.name })
    router.replace('/projects')
  } catch (err) {
    toast.error('删除项目失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

// ---------- 「⋯」更多菜单 ----------
const MORE_ITEMS: MenuItem[] = [
  { key: 'rename-project', label: '重命名项目', icon: 'pencil' },
  {
    key: 'delete-project',
    label: '删除项目',
    icon: 'trash',
    danger: true,
    dividerBefore: true,
    confirm: `删除项目「${''}」？`,
  },
  { key: 'import-doc', label: '导入 API 文档', icon: 'upload', dividerBefore: true },
  { key: 'go-projects', label: '项目列表', icon: 'folder' },
  { key: 'go-graphql', label: 'GraphQL 工作台', icon: 'code' },
]

function openSidebarMore(event: MouseEvent): void {
  if (!store.project) return
  const el = event.currentTarget as HTMLElement
  const items = MORE_ITEMS.map((m) =>
    m.key === 'delete-project' ? { ...m, confirm: `删除项目「${store.project?.name}」？` } : m,
  )
  menuRef.value?.openAt(el, items, 'right')
}

function createFolderAtRoot(): void {
  treeRef.value?.startEdit('create-folder', { parentId: null })
}

const DOCS_ITEMS: MenuItem[] = [
  { key: 'import', label: '导入文档', icon: 'upload' },
  { key: 'export', label: '导出 OpenAPI', icon: 'download' },
]

function openDocsMenu(): void {
  if (docsBtn.value) menuRef.value?.openAt(docsBtn.value, DOCS_ITEMS, 'right')
}

function onDocsSelect(item: MenuItem): void {
  if (item.key === 'import') showDocImport.value = true
  else void exportOpenapi()
}

function openMockMenu(): void {
  if (!mockBtn.value) return
  menuRef.value?.openAt(mockBtn.value, [
    {
      key: 'toggle',
      label: mockBusy.value ? '处理中…' : mockAddress.value ? '停止 Mock' : '启动 Mock',
      icon: mockAddress.value ? 'stop' : 'play',
      disabled: mockBusy.value,
    },
    { key: 'rules', label: 'Mock 规则', icon: 'shield', dividerBefore: true },
  ], 'right')
}

function onMockSelect(item: MenuItem): void {
  if (item.key === 'toggle') void toggleMock()
  else if (item.key === 'rules') showMockRules.value = true
}

function onMenuSelect(item: MenuItem): void {
  if (item.key === 'rename-project') openRenameProject()
  else if (item.key === 'import-doc') showDocImport.value = true
  else if (item.key === 'go-projects') void router.push('/projects')
  else if (item.key === 'go-graphql') void router.push('/graphql')
  else if (item.key.startsWith('switch-project:')) void switchProject(item.key.slice('switch-project:'.length))
  else if (item.key === 'new-project') openCreateProject()
  else if (item.key === 'import' || item.key === 'export') onDocsSelect(item)
  else onMockSelect(item)
}

function onMenuConfirm(item: MenuItem): void {
  if (item.key === 'delete-project') void confirmDeleteProject()
}

onMounted(() => {
  load()
  refreshMockStatus()
  void store.loadHistories()
})

onBeforeUnmount(() => {
  menuRef.value?.close()
})
</script>

<template>
  <div class="workspace">
    <div v-if="store.project" class="top-bar">
      <div class="tb-region tb-left">
        <Brand title="RustFox" class="tb-brand" />
      </div>
      <span class="tb-divider" aria-hidden="true"></span>

      <div class="tb-region tb-middle">
        <span class="mock-status" :class="{ on: mockAddress }">
          <span class="mock-dot"></span>
          {{ mockAddress ? 'Mock 运行中' : 'Mock 未运行' }}
          <span v-if="mockAddress" class="mock-addr">{{ mockAddress }}</span>
        </span>
      </div>
      <span class="tb-divider" aria-hidden="true"></span>

      <div class="tb-region tb-right">
        <button ref="docsBtn" class="rf-btn rf-btn-sm tb-action" type="button" @click="openDocsMenu">
          <Icon name="file" :size="13" /> 文档 (Docs) <Icon name="chevron-down" :size="12" />
        </button>
        <button ref="mockBtn" class="rf-btn rf-btn-sm tb-action" type="button" @click="openMockMenu">
          <span class="mock-dot" :class="{ on: mockAddress }"></span>
          Mock <Icon name="chevron-down" :size="12" />
        </button>
        <EnvironmentBar />
        <IconButton class="tb-settings" name="settings" :size="15" title="设置" @click="showSettings = true" />
      </div>
    </div>

    <div class="workspace-body">
      <aside class="rf-sidebar">
        <div class="sidebar-head">
          <button
            ref="projectBtn"
            class="project-switcher"
            type="button"
            title="切换项目"
            @click="openProjectMenu"
          >
            <span class="ps-name">{{ store.project?.name ?? '工作区' }}</span>
            <Icon name="chevron-down" :size="13" class="ps-caret" />
          </button>
          <div class="sidebar-actions">
            <IconButton
              v-if="sidebarTab === 'collections'"
              name="folder-plus"
              :size="16"
              title="新建文件夹"
              @click="createFolderAtRoot"
            />
            <IconButton
              v-if="sidebarTab === 'collections'"
              name="terminal"
              :size="16"
              title="导入 cURL"
              @click="openCurlImport(null)"
            />
            <IconButton
              name="more-horizontal"
              :size="16"
              title="更多操作（重命名 / 删除项目）"
              @click="openSidebarMore"
            />
          </div>
        </div>
        <Tabs v-model="sidebarTab" :tabs="sidebarTabs" size="sm" class="sidebar-tabs" />
        <div v-show="sidebarTab === 'collections'" class="sidebar-collections">
          <div class="sidebar-search">
            <Icon name="search" :size="13" class="ss-icon" />
            <input
              v-model="apiSearch"
              class="ss-input"
              type="text"
              placeholder="搜索接口名称或路径..."
              spellcheck="false"
            />
            <button
              v-if="apiSearch"
              class="ss-clear"
              type="button"
              title="清除搜索"
              aria-label="清除搜索"
              @click="apiSearch = ''"
            >
              <Icon name="x" :size="12" />
            </button>
          </div>
          <div v-if="store.loadError" class="rf-inline-error" role="alert">
            <span class="rf-inline-error-text">加载失败：{{ store.loadError }}</span>
            <button class="rf-btn rf-btn-sm" type="button" :disabled="loading" @click="load">
              {{ loading ? '重试中…' : '重试' }}
            </button>
          </div>
          <div v-else class="tree-wrap">
            <EndpointTree ref="treeRef" :folder-id="null" :search="apiSearch" @import-curl="openCurlImport" />
          </div>
        </div>
        <HistoryPanel v-if="sidebarTab === 'history'" class="sidebar-history" />
      </aside>
      <main class="rf-main">
        <TabBar v-if="store.openTabs.length" />
        <EndpointEditor />
      </main>
    </div>

    <Menu ref="menuRef" @select="onMenuSelect" @confirm="onMenuConfirm" />

    <Modal v-model:open="showCreateProject" title="新建项目" width="420px" @close="showCreateProject = false">
      <div class="form-field">
        <label class="form-label">项目名称</label>
        <input
          v-model="newProjectName"
          class="rf-input"
          :class="{ 'rf-input-error': projectFormError }"
          placeholder="例如：电子商务后端 API"
          maxlength="60"
          spellcheck="false"
          @input="projectFormError = null"
          @keyup.enter="confirmCreateProject"
        />
      </div>
      <div class="form-field">
        <label class="form-label">描述（可选）</label>
        <textarea
          v-model="newProjectDesc"
          class="rf-textarea"
          :maxlength="200"
          placeholder="项目用途与说明…"
          rows="3"
        ></textarea>
      </div>
      <p v-if="projectFormError" class="rf-field-error" role="alert">{{ projectFormError }}</p>
      <template #footer>
        <button class="rf-btn" type="button" @click="showCreateProject = false">取消</button>
        <button class="rf-btn rf-btn-primary" type="button" :disabled="api.pending.value" @click="confirmCreateProject">
          创建
        </button>
      </template>
    </Modal>

    <Modal v-model:open="renamingProject" title="重命名项目" width="420px" @close="renamingProject = false">
      <div class="form-field">
        <label class="form-label">项目名称</label>
        <input
          v-model="renameProjectName"
          class="rf-input"
          :class="{ 'rf-input-error': projectFormError }"
          placeholder="项目名称"
          maxlength="60"
          spellcheck="false"
          @input="projectFormError = null"
          @keyup.enter="confirmRenameProject"
        />
      </div>
      <p v-if="projectFormError" class="rf-field-error" role="alert">{{ projectFormError }}</p>
      <template #footer>
        <button class="rf-btn" type="button" @click="renamingProject = false">取消</button>
        <button class="rf-btn rf-btn-primary" type="button" :disabled="api.pending.value" @click="confirmRenameProject">
          保存
        </button>
      </template>
    </Modal>

    <CurlImportDialog
      v-if="showCurlImport"
      :folder-id="curlFolderId"
      @close="showCurlImport = false"
    />
    <ImportDialog v-if="showDocImport" @close="showDocImport = false" />
    <MockRuleDialog v-if="showMockRules" @close="showMockRules = false" />
    <SettingsDialog v-if="showSettings" @close="showSettings = false" />
  </div>
</template>

<style scoped>
.workspace {
  display: flex;
  flex-direction: column;
  height: 100%;
  box-sizing: border-box;
}

.workspace-body {
  flex: 1;
  display: flex;
  min-height: 0;
}

/* ---- 顶栏：左（品牌）/ 中（状态）/ 右（操作 + 环境）三区 ---- */
.top-bar {
  display: flex;
  align-items: center;
  height: 48px;
  padding: 0 12px;
  gap: 12px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--rf-border);
  background: var(--rf-bg-panel);
}

.tb-region {
  display: flex;
  align-items: center;
  min-width: 0;
}

.tb-left {
  flex: 0 1 auto;
}

.tb-brand {
  width: 140px;
}

.tb-middle {
  justify-content: center;
}

.tb-right {
  margin-left: auto;
  justify-content: flex-end;
  gap: 8px;
  flex: 1;
}

.tb-divider {
  width: 1px;
  height: 20px;
  flex-shrink: 0;
  background: var(--rf-border);
  opacity: 0.55;
}

.tb-inner-divider {
  width: 1px;
  height: 20px;
  flex-shrink: 0;
  margin: 0 2px;
  background: var(--rf-border);
  opacity: 0.55;
}

/* ---- 顶栏右区操作：统一 h-8、border-white/10、rounded-lg、gap-2 ---- */
.tb-right .tb-action {
  height: 32px;
  padding: 0 12px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: var(--bg-hover);
  font-size: 12px;
  gap: 6px;
  color: var(--text-2);
}
.tb-right .tb-action:hover:not(:disabled) {
  background: var(--bg-active);
  border-color: rgba(255, 255, 255, 0.2);
  color: var(--text-1);
}
.tb-right .tb-settings {
  width: 32px;
  height: 32px;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  background: var(--bg-hover);
}
.tb-right .tb-settings:hover:not(:disabled) {
  background: var(--bg-active);
}

/* ---- 状态指示（中区） ---- */
.mock-status {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  font-size: 12px;
  color: var(--rf-text-muted);
  white-space: nowrap;
}

.mock-addr {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--rf-text-secondary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 200px;
}

.mock-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--rf-text-muted);
}

.mock-status.on {
  color: var(--rf-success);
}

.mock-status.on .mock-dot {
  background: var(--rf-success);
  box-shadow: 0 0 0 3px var(--rf-success-tint);
}

.mock-dot.on {
  background: var(--rf-success);
  box-shadow: 0 0 0 3px var(--rf-success-tint);
}

.rf-sidebar {
  width: 300px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--rf-border);
  background: var(--rf-bg-panel);
  overflow: hidden;
}

/* ---- 统一头部栏：h-10、下边框、左=项目切换器 / 右=动作图标组 ---- */
.sidebar-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  height: 40px;
  padding: 8px 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  flex-shrink: 0;
}

.sidebar-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

/* 项目切换器：静默文本 + ▾，hover 淡底提示可点 */
.project-switcher {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  max-width: 180px;
  padding: 4px 6px;
  margin-left: -6px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-1);
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease);
}
.project-switcher:hover {
  background: rgba(255, 255, 255, 0.05);
  color: var(--text-1);
}
.project-switcher:active {
  background: rgba(255, 255, 255, 0.09);
}

.ps-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ps-caret {
  flex-shrink: 0;
  color: var(--text-3);
  transition: transform var(--dur) var(--ease);
}

.rf-heading {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 接口搜索栏：h-7、bg-white/5、rounded、px-2.5、text-xs、text-gray-300 */
.sidebar-search {
  display: flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  margin: 8px 12px 0;
  padding: 0 8px;
  flex-shrink: 0;
  border: 1px solid rgba(255, 255, 255, 0.06);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.05);
  transition:
    border-color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.sidebar-search:focus-within {
  border-color: rgba(168, 85, 247, 0.5);
  background: rgba(255, 255, 255, 0.07);
}

.ss-icon {
  flex-shrink: 0;
  color: var(--text-3);
  opacity: 0.7;
}

.ss-input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  font-family: inherit;
  font-size: 12px;
  color: #d1d5db;
}
.ss-input::placeholder {
  color: rgba(209, 213, 219, 0.45);
}

.ss-clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  border: none;
  border-radius: 4px;
  background: none;
  color: var(--text-3);
  cursor: pointer;
  padding: 0;
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease);
}
.ss-clear:hover {
  background: var(--bg-hover);
  color: var(--text-1);
}

.tree-wrap {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px 12px;
}

/* ---- 侧栏页签（接口目录 / 请求历史） ---- */
.sidebar-tabs {
  flex-shrink: 0;
  padding: 0 12px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.05);
}

.sidebar-collections {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.sidebar-history {
  flex: 1;
  min-height: 0;
}

.rf-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--rf-bg);
  overflow: hidden;
}

.rf-inline-error {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--rf-radius-sm);
  background: var(--rf-danger-tint);
  border: 1px solid rgba(239, 68, 68, 0.35);
}

.rf-inline-error-text {
  font-size: 12.5px;
  color: var(--rf-danger);
  word-break: break-all;
}

/* ---- 项目表单（新建 / 重命名） ---- */
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

.rf-input-error {
  border-color: var(--danger) !important;
}
.rf-input-error:focus {
  border-color: var(--danger) !important;
  box-shadow: 0 0 0 2px var(--danger-tint) !important;
}
</style>
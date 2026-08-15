<script lang="ts">
/**
 * 共享拖拽状态（模块级 reactive，递归实例间可见）。
 * 采用指针事件实现（pointerdown/move/up + elementFromPoint 命中测试），
 * 规避 WKWebView 对 HTML5 Drag & Drop（dragover/drop/dataTransfer）支持不稳的问题。
 */
import { reactive } from 'vue'

export interface DndTarget {
  drop: 'folder' | 'before' | 'after' | 'root'
  id: string | null
  index: number
}

export const dndState = reactive({
  active: false,
  kind: '' as 'folder' | 'endpoint' | '',
  id: '',
  target: null as DndTarget | null,
})

/** 拖拽结束后抑制紧随其后的 click（避免误打开接口/文件夹）。 */
let suppressClick = false

export function consumeSuppressClick(): boolean {
  const s = suppressClick
  suppressClick = false
  return s
}

export function setSuppressClick(v: boolean): void {
  suppressClick = v
}
</script>

<script setup lang="ts">
/**
 * EndpointTree：项目接口树（递归）。props.folderId 为 null 时渲染根节点。
 *
 * - 文件夹节点：SVG 图标 + chevron 旋转动画、展开/收起；
 * - 接口节点：方法=彩色 mono 文本、脏标记圆点；
 * - hover 显现「⋯」更多按钮 → 弹出动作菜单（新建/导入/重命名/删除，删除带行内确认）；
 * - 行高 28px、hover/选中态、缩进引导线；
 * - 新建/重命名用行内输入（Enter 提交 / Esc 取消）；
 * - 根级新建文件夹由侧栏头部按钮触发（defineExpose(startEdit)）；
 * - 拖拽移动：跨实例共享载荷，dragover 显式 dropEffect=move。
 */
import { computed, ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useToast } from '../composables/useToast'
import { escapeHtml } from '../utils/highlight'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import Menu from './ui/Menu.vue'
import type { MenuItem } from './ui/Menu.vue'
import type { Endpoint, Folder } from '../types/foxApi'

const props = withDefaults(
  defineProps<{ folderId: string | null; search?: string }>(),
  { search: '' },
)
const emit = defineEmits<{ importCurl: [folderId: string | null] }>()

const store = useWorkspaceStore()
const toast = useToast()

const expanded = ref<Set<string>>(new Set())
const editing = ref<{
  kind: 'create-folder' | 'rename-folder' | 'rename-endpoint'
  id?: string
  parentId?: string | null
} | null>(null)
const editValue = ref('')

// ---------- 接口搜索（实时过滤） ----------
const query = computed(() => props.search.trim().toLowerCase())
const searchActive = computed(() => query.value.length > 0)

function endpointMatches(e: Endpoint): boolean {
  if (!searchActive.value) return true
  const name = (e.name || e.path).toLowerCase()
  return name.includes(query.value) || e.path.toLowerCase().includes(query.value)
}

/** 文件夹（或其子孙）是否包含匹配的接口。 */
function folderHasMatch(folderId: string): boolean {
  if (store.endpoints.some((e) => e.folder_id === folderId && endpointMatches(e))) return true
  return store.folders.some((f) => f.parent_id === folderId && folderHasMatch(f.id))
}

/** 命中子串包 <mark> 高亮（已转义，安全注入 v-html）。 */
function highlightName(text: string): string {
  const q = query.value
  if (!q) return escapeHtml(text)
  const lower = text.toLowerCase()
  let out = ''
  let i = 0
  for (;;) {
    const idx = lower.indexOf(q, i)
    if (idx === -1) {
      out += escapeHtml(text.slice(i))
      break
    }
    out += `${escapeHtml(text.slice(i, idx))}<mark class="tree-hit">${escapeHtml(text.slice(idx, idx + q.length))}</mark>`
    i = idx + q.length
  }
  return out
}

const childFolders = computed(() =>
  store.folders.filter(
    (f) => f.parent_id === props.folderId && (!searchActive.value || folderHasMatch(f.id)),
  ),
)
const childEndpoints = computed(() =>
  store.endpoints.filter((e) => e.folder_id === props.folderId && endpointMatches(e)),
)

function toggleFolder(id: string): void {
  const next = new Set(expanded.value)
  next.has(id) ? next.delete(id) : next.add(id)
  expanded.value = next
}

// ---------- 拖拽排序 / 移动（指针事件实现，规避 WKWebView HTML5 DnD 缺陷） ----------
let dragStart = { x: 0, y: 0 }
let folderExpandTimer: number | null = null

const ghostPos = ref({ x: 0, y: 0 })
const ghostInfo = computed(() => {
  if (!dndState.active) return null
  if (dndState.kind === 'endpoint') {
    const e = store.endpoints.find((x) => x.id === dndState.id)
    if (!e) return null
    return { method: e.method, title: (e.name || e.path).slice(0, 5) }
  }
  const f = store.folders.find((x) => x.id === dndState.id)
  if (!f) return null
  return { method: null, title: f.name.slice(0, 5) }
})

function onRowPointerDown(event: PointerEvent, kind: 'folder' | 'endpoint', id: string): void {
  if (event.button !== 0 || dndState.active || editing.value) return
  dragStart = { x: event.clientX, y: event.clientY }
  dndState.kind = kind
  dndState.id = id
  dndState.target = null
  window.addEventListener('pointermove', onPointerMove)
  window.addEventListener('pointerup', onPointerUp)
  window.addEventListener('pointercancel', onPointerCancel)
}

function endDrag(): void {
  window.removeEventListener('pointermove', onPointerMove)
  window.removeEventListener('pointerup', onPointerUp)
  window.removeEventListener('pointercancel', onPointerCancel)
  document.body.classList.remove('dragging-dnd')
  clearFolderExpand()
  dndState.active = false
  dndState.kind = ''
  dndState.id = ''
  dndState.target = null
}

function onPointerCancel(): void {
  endDrag()
}

/** 命中测试：最近的行（folder / before/after）或树根（append 到该层末尾）。 */
function hitTest(x: number, y: number): DndTarget | null {
  const el = document.elementFromPoint(x, y) as HTMLElement | null
  if (!el) return null
  const row = el.closest<HTMLElement>('[data-dnd-kind]')
  if (row) {
    const kind = row.dataset.dndKind
    const id = row.dataset.dndId ?? null
    if (kind === 'folder') return { drop: 'folder', id, index: Number.MAX_SAFE_INTEGER }
    if (kind === 'endpoint') {
      const rect = row.getBoundingClientRect()
      const before = y < rect.top + rect.height / 2
      return {
        drop: before ? 'before' : 'after',
        id,
        index: Number(row.dataset.dndIndex ?? '0'),
      }
    }
    return null
  }
  const root = el.closest<HTMLElement>('[data-dnd-tree-root]')
  if (root)
    return { drop: 'root', id: root.dataset.dndTreeRoot || null, index: Number.MAX_SAFE_INTEGER }
  return null
}

function clearFolderExpand(): void {
  if (folderExpandTimer !== null) {
    window.clearTimeout(folderExpandTimer)
    folderExpandTimer = null
  }
}

/** 悬停在关闭的文件夹上 >500ms 时自动展开（方便拖入其中）。 */
function scheduleFolderExpand(id: string | null): void {
  clearFolderExpand()
  if (!id || searchActive.value || expanded.value.has(id)) return
  folderExpandTimer = window.setTimeout(() => {
    expanded.value.add(id)
    folderExpandTimer = null
  }, 500)
}

function onPointerMove(event: PointerEvent): void {
  if (!dndState.active) {
    if (Math.hypot(event.clientX - dragStart.x, event.clientY - dragStart.y) < 5) return
    dndState.active = true
    document.body.classList.add('dragging-dnd')
  }
  ghostPos.value = { x: event.clientX, y: event.clientY }
  let t = hitTest(event.clientX, event.clientY)
  if (t && t.id === dndState.id && (t.drop === 'before' || t.drop === 'after' || t.drop === 'folder'))
    t = null
  if (t?.drop === 'folder') scheduleFolderExpand(t.id)
  else clearFolderExpand()
  dndState.target = t
}

async function onPointerUp(): Promise<void> {
  const wasActive = dndState.active
  const d = { kind: dndState.kind, id: dndState.id, target: dndState.target }
  endDrag()
  setSuppressClick(wasActive)
  if (!wasActive || !d.target || !d.id) return
  const t = d.target
  try {
    let targetFolder: string | null
    let index: number
    if (t.drop === 'folder') {
      if (t.id === d.id) return
      targetFolder = t.id
      index = Number.MAX_SAFE_INTEGER
    } else if (t.drop === 'before' || t.drop === 'after') {
      const targetEp = store.endpoints.find((x) => x.id === t.id)
      if (!targetEp || targetEp.id === d.id) return
      targetFolder = targetEp.folder_id
      index = targetEp.sort_order + (t.drop === 'after' ? 1 : 0)
    } else {
      targetFolder = t.id
      index = Number.MAX_SAFE_INTEGER
    }
    if (d.kind === 'folder') {
      if (targetFolder !== d.id) {
        await store.moveFolder(d.id, targetFolder, index)
        toast.success('文件夹已移动')
      }
    } else {
      await store.moveEndpoint(d.id, targetFolder, index)
      toast.success('接口已移动')
    }
  } catch (err) {
    console.error('[EndpointTree.dnd]', err)
    toast.error('移动失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

function isOverFolder(id: string): boolean {
  return dndState.active && dndState.target?.drop === 'folder' && dndState.target.id === id
}

function isInsertBefore(id: string): boolean {
  return dndState.active && dndState.target?.drop === 'before' && dndState.target.id === id
}

function isInsertAfter(id: string): boolean {
  return dndState.active && dndState.target?.drop === 'after' && dndState.target.id === id
}

function isDraggingSrc(id: string): boolean {
  return dndState.active && dndState.id === id
}

/** 拖拽结束后的残余 click 在树根捕获层消费掉。 */
function onTreeClickCapture(event: Event): void {
  if (consumeSuppressClick()) event.stopPropagation()
}

function startEdit(
  kind: 'create-folder' | 'rename-folder' | 'rename-endpoint',
  opts?: { id?: string; parentId?: string | null },
): void {
  editing.value = { kind, ...opts }
  editValue.value = ''
  if (kind === 'rename-folder' && opts?.id) {
    editValue.value = store.folders.find((f) => f.id === opts.id)?.name ?? ''
  }
  if (kind === 'rename-endpoint' && opts?.id) {
    editValue.value = store.endpoints.find((e) => e.id === opts.id)?.name ?? ''
  }
}

function cancelEdit(): void {
  editing.value = null
  editValue.value = ''
}

defineExpose({ startEdit })

async function commitEdit(): Promise<void> {
  const ed = editing.value
  if (!ed) return
  const name = editValue.value.trim()
  if (!name) {
    cancelEdit()
    return
  }
  const now = new Date().toISOString()
  try {
    if (ed.kind === 'create-folder') {
      await store.saveFolder({
        id: crypto.randomUUID(),
        project_id: store.project!.id,
        parent_id: ed.parentId ?? null,
        name,
        sort_order: 0,
        created_at: now,
        updated_at: now,
      })
      if (ed.parentId) expanded.value.add(ed.parentId)
    } else if (ed.kind === 'rename-folder' && ed.id) {
      const f = store.folders.find((x) => x.id === ed.id)
      if (f) await store.saveFolder({ ...f, name, updated_at: now })
    } else if (ed.kind === 'rename-endpoint' && ed.id) {
      await store.renameEndpoint(ed.id, name)
    }
  } catch (err) {
    console.error('[EndpointTree.commitEdit]', err)
  } finally {
    cancelEdit()
  }
}

async function removeFolder(id: string): Promise<void> {
  try {
    await store.deleteFolder(id)
  } catch (err) {
    console.error('[EndpointTree.removeFolder]', err)
  }
}

async function removeEndpoint(e: Endpoint): Promise<void> {
  try {
    await store.deleteEndpoint(e.id)
  } catch (err) {
    console.error('[EndpointTree.removeEndpoint]', err)
  }
}

async function duplicate(e: Endpoint): Promise<void> {
  try {
    await store.duplicateEndpoint(e.id)
  } catch (err) {
    console.error('[EndpointTree.duplicate]', err)
  }
}

// ---------- 行内动作菜单 ----------
const menu = ref<InstanceType<typeof Menu> | null>(null)
const menuTarget = ref<{ kind: 'folder' | 'endpoint'; id: string } | null>(null)

function openFolderMenu(event: MouseEvent, f: Folder): void {
  menuTarget.value = { kind: 'folder', id: f.id }
  menu.value?.openAt(event.currentTarget as HTMLElement, [
    { key: 'subfolder', label: '新建子文件夹', icon: 'folder-plus' },
    { key: 'endpoint', label: '新建接口', icon: 'file-plus' },
    { key: 'import', label: '导入 cURL', icon: 'terminal', dividerBefore: true },
    { key: 'rename', label: '重命名', icon: 'pencil', dividerBefore: true },
    {
      key: 'delete',
      label: '删除文件夹',
      icon: 'trash',
      danger: true,
      confirm: `删除文件夹「${f.name}」及其全部子文件夹/接口？`,
    },
  ])
}

function openEndpointMenu(event: MouseEvent, e: Endpoint): void {
  menuTarget.value = { kind: 'endpoint', id: e.id }
  menu.value?.openAt(event.currentTarget as HTMLElement, [
    { key: 'copy', label: '复制', icon: 'copy' },
    { key: 'rename', label: '重命名', icon: 'pencil' },
    {
      key: 'delete',
      label: '删除接口',
      icon: 'trash',
      danger: true,
      dividerBefore: true,
      confirm: `删除接口「${e.name || e.path}」？`,
    },
  ])
}

function onMenuSelect(item: MenuItem): void {
  const target = menuTarget.value
  if (!target) return
  if (target.kind === 'folder') {
    if (item.key === 'subfolder') startEdit('create-folder', { parentId: target.id })
    else if (item.key === 'endpoint') store.openNewEndpoint(target.id)
    else if (item.key === 'import') emit('importCurl', target.id)
    else if (item.key === 'rename') startEdit('rename-folder', { id: target.id })
  } else {
    if (item.key === 'copy') duplicate(store.endpoints.find((x) => x.id === target.id)!)
    else if (item.key === 'rename') startEdit('rename-endpoint', { id: target.id })
  }
}

function onMenuConfirm(item: MenuItem): void {
  const target = menuTarget.value
  if (!target) return
  if (item.key !== 'delete') return
  if (target.kind === 'folder') removeFolder(target.id)
  else {
    const ep = store.endpoints.find((x) => x.id === target.id)
    if (ep) removeEndpoint(ep)
  }
}
</script>

<template>
  <div class="tree" :data-dnd-tree-root="folderId ?? ''" @click.capture="onTreeClickCapture">
    <template v-for="f in childFolders" :key="f.id">
      <div
        class="tree-row"
        :class="{ 'dnd-over': isOverFolder(f.id) }"
        data-dnd-kind="folder"
        :data-dnd-id="f.id"
        @pointerdown="onRowPointerDown($event, 'folder', f.id)"
      >
        <span
          class="tree-chevron"
          :class="{ open: expanded.has(f.id) || searchActive }"
          @click="toggleFolder(f.id)"
        >
          <Icon name="chevron-right" :size="13" />
        </span>
        <template v-if="editing?.kind === 'rename-folder' && editing.id === f.id">
          <input
            v-model="editValue"
            class="rf-input rf-input-sm tree-input"
            autofocus
            @keyup.enter="commitEdit"
            @keyup.esc="cancelEdit"
            @blur="commitEdit"
          />
        </template>
        <template v-else>
          <span class="tree-folder-icon" @click="toggleFolder(f.id)">
            <Icon :name="expanded.has(f.id) || searchActive ? 'folder-open' : 'folder'" :size="15" />
          </span>
          <span class="tree-name folder" @click="toggleFolder(f.id)">{{ f.name }}</span>
          <span class="tree-actions">
            <IconButton name="more-horizontal" :size="13" title="更多操作" @click="openFolderMenu($event, f)" />
          </span>
        </template>
      </div>
      <div v-show="expanded.has(f.id) || searchActive" class="tree-children">
        <EndpointTree
          :folder-id="f.id"
          :search="props.search"
          @import-curl="$emit('importCurl', $event)"
        />
      </div>
    </template>

    <div v-if="editing?.kind === 'create-folder' && editing.parentId === folderId" class="tree-row">
      <input
        v-model="editValue"
        class="rf-input rf-input-sm tree-input"
        placeholder="文件夹名称"
        autofocus
        @keyup.enter="commitEdit"
        @keyup.esc="cancelEdit"
        @blur="commitEdit"
      />
    </div>

    <template v-for="(e, i) in childEndpoints" :key="e.id">
      <div
        class="tree-row"
        :class="{
          active: store.activeTabId === e.id,
          'insert-before': isInsertBefore(e.id),
          'insert-after': isInsertAfter(e.id),
          'dragging-src': isDraggingSrc(e.id),
        }"
        data-dnd-kind="endpoint"
        :data-dnd-id="e.id"
        :data-dnd-index="i"
        @pointerdown="onRowPointerDown($event, 'endpoint', e.id)"
      >
        <template v-if="editing?.kind === 'rename-endpoint' && editing.id === e.id">
          <input
            v-model="editValue"
            class="rf-input rf-input-sm tree-input"
            autofocus
            @keyup.enter="commitEdit"
            @keyup.esc="cancelEdit"
            @blur="commitEdit"
          />
        </template>
        <template v-else>
          <span class="tree-chevron spacer"></span>
          <span class="tree-method" :class="`rf-method-${e.method.toLowerCase()}`">{{ e.method }}</span>
          <span class="tree-name" :class="{ dirty: store.isDirty(e.id) }" @click="store.openEndpoint(e)">
            <span class="tree-name-text" v-html="highlightName(e.name || e.path)"></span>
            <Icon v-if="store.isDirty(e.id)" class="tree-dirty" name="dot" :size="6" />
          </span>
          <span class="tree-actions">
            <IconButton name="more-horizontal" :size="13" title="更多操作" @click="openEndpointMenu($event, e)" />
          </span>
        </template>
      </div>
    </template>

    <p
      v-if="folderId === null && searchActive && !childFolders.length && !childEndpoints.length"
      class="tree-empty"
    >
      未找到匹配接口
    </p>
  </div>

  <Menu ref="menu" @select="onMenuSelect" @confirm="onMenuConfirm" />

  <Teleport to="body">
    <div
      v-if="folderId === null && dndState.active && ghostInfo"
      class="dnd-ghost"
      :style="{ left: ghostPos.x + 'px', top: ghostPos.y + 'px' }"
    >
      <span v-if="ghostInfo.method" class="rf-method" :class="`rf-method-${ghostInfo.method.toLowerCase()}`">
        {{ ghostInfo.method }}
      </span>
      <Icon v-else name="folder" :size="13" />
      <span class="ghost-title">{{ ghostInfo.title }}</span>
    </div>
  </Teleport>
</template>

<style scoped>
.tree {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.tree-row {
  display: flex;
  align-items: center;
  gap: 6px;
  min-height: 28px;
  padding: 2px 6px;
  border-radius: var(--radius);
  cursor: default;
  transition: background var(--dur) var(--ease);
}
.tree-row:hover {
  background: var(--bg-hover);
}
.tree-row.active {
  background: var(--accent-tint);
}
/* 拖入文件夹：细虚线描边 + 浅色高亮 */
.tree-row.dnd-over {
  background: var(--accent-tint);
  outline: 1px dashed var(--accent);
  outline-offset: -1px;
}
/* 拖拽中的原行淡出（配合浮空小卡片，去掉厚重描边） */
.tree-row.dragging-src {
  opacity: 0.35;
}
/* 接口间精确插入线：悬停上半 → 上方线；悬停下半 → 下方线 */
.tree-row.insert-before {
  border-top: 2px solid var(--accent);
}
.tree-row.insert-after {
  border-bottom: 2px solid var(--accent);
}
.tree-row.active .tree-name {
  color: var(--text-1);
}

.tree-chevron {
  width: 16px;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--text-3);
  cursor: pointer;
  user-select: none;
  transition: color var(--dur) var(--ease);
}
.tree-chevron:hover {
  color: var(--text-1);
}
.tree-chevron svg {
  transition: transform var(--dur) var(--ease);
}
.tree-chevron.open svg {
  transform: rotate(90deg);
}
.tree-chevron.spacer {
  cursor: default;
}

.tree-folder-icon {
  display: inline-flex;
  align-items: center;
  color: var(--post);
  cursor: pointer;
  flex-shrink: 0;
}

.tree-name {
  flex: 1;
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12.5px;
  color: var(--text-1);
  cursor: pointer;
}
.tree-name.folder {
  font-weight: 600;
}
.tree-name-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 搜索命中高亮（v-html 注入，需 :deep） */
:deep(.tree-hit) {
  padding: 0 1px;
  border-radius: 3px;
  background: rgba(250, 204, 21, 0.32);
  color: #fde68a;
}
.tree-dirty {
  color: var(--warning);
  flex-shrink: 0;
}

.tree-empty {
  margin: 10px 0 0;
  font-size: 12px;
  color: var(--text-3);
  text-align: center;
}

.tree-method {
  width: 44px;
  flex-shrink: 0;
  text-align: left;
}

.tree-actions {
  display: none;
  gap: 1px;
  flex-shrink: 0;
}
.tree-row:hover .tree-actions {
  display: inline-flex;
}

.tree-input {
  flex: 1;
  min-width: 0;
}

/* 缩进引导线 */
.tree-children {
  padding-left: 16px;
  position: relative;
}
.tree-children::before {
  content: '';
  position: absolute;
  left: 7px;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--border);
}

/* 拖拽浮空小卡片：方法标签 + 标题前 5 字符，跟随光标 */
.dnd-ghost {
  position: fixed;
  transform: translate(14px, 18px);
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 4px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  background: var(--bg-elevated);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.4);
  opacity: 0.8;
  pointer-events: none;
  z-index: 9999;
  white-space: nowrap;
}
.dnd-ghost .ghost-title {
  font-size: 12px;
  color: var(--text-1);
}
</style>

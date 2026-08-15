<script setup lang="ts">
/**
 * EndpointTree：项目接口树（递归）。props.folderId 为 null 时渲染根节点。
 *
 * - 文件夹节点：展开/收起、新建子文件夹、新建接口、重命名、删除（级联）；
 * - 接口节点：点击打开标签页（草稿）、重命名、复制、删除；
 * - 新建/重命名用行内输入（Enter 提交 / Esc 取消）；
 * - cURL 导入入口冒泡到 WorkspaceView（需要弹窗）。
 */
import { computed, ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import type { Endpoint } from '../types/foxApi'

const props = defineProps<{ folderId: string | null }>()
defineEmits<{ importCurl: [folderId: string | null] }>()

const store = useWorkspaceStore()

const expanded = ref<Set<string>>(new Set())
const editing = ref<{
  kind: 'create-folder' | 'rename-folder' | 'rename-endpoint'
  id?: string
  parentId?: string | null
} | null>(null)
const editValue = ref('')

const childFolders = computed(() => store.folders.filter((f) => f.parent_id === props.folderId))
const childEndpoints = computed(() => store.endpoints.filter((e) => e.folder_id === props.folderId))

function toggleFolder(id: string): void {
  const next = new Set(expanded.value)
  next.has(id) ? next.delete(id) : next.add(id)
  expanded.value = next
}

// ---------- 拖拽排序 / 移动 ----------
/** 拖拽载荷：kind + id，落点按类型分派（文件夹内 / 兄弟之前 / 根末尾）。 */
function onDragStart(event: DragEvent, kind: 'folder' | 'endpoint', id: string): void {
  event.dataTransfer?.setData('text/plain', `${kind}:${id}`)
  event.dataTransfer!.effectAllowed = 'move'
}

function parseDrag(event: DragEvent): { kind: 'folder' | 'endpoint'; id: string } | null {
  const raw = event.dataTransfer?.getData('text/plain')
  if (!raw) return null
  const [kind, id] = raw.split(':')
  return kind === 'folder' || kind === 'endpoint' ? { kind, id } : null
}

async function onDropIntoFolder(event: DragEvent, folderId: string): Promise<void> {
  event.preventDefault()
  const drag = parseDrag(event)
  if (!drag) return
  try {
    if (drag.kind === 'folder') {
      if (drag.id !== folderId) await store.moveFolder(drag.id, folderId, Number.MAX_SAFE_INTEGER)
    } else {
      await store.moveEndpoint(drag.id, folderId, Number.MAX_SAFE_INTEGER)
    }
  } catch (err) {
    console.error('[EndpointTree.dropIntoFolder]', err)
  }
}

async function onDropBeforeEndpoint(event: DragEvent, endpointId: string, index: number): Promise<void> {
  event.preventDefault()
  const drag = parseDrag(event)
  if (!drag) return
  try {
    if (drag.kind === 'endpoint') {
      if (drag.id === endpointId) return
      await store.moveEndpoint(drag.id, props.folderId, index)
    } else {
      await store.moveFolder(drag.id, props.folderId, index)
    }
  } catch (err) {
    console.error('[EndpointTree.dropBeforeEndpoint]', err)
  }
}

async function onDropToRoot(event: DragEvent): Promise<void> {
  event.preventDefault()
  const drag = parseDrag(event)
  if (!drag) return
  try {
    if (drag.kind === 'folder') {
      await store.moveFolder(drag.id, null, Number.MAX_SAFE_INTEGER)
    } else {
      await store.moveEndpoint(drag.id, null, Number.MAX_SAFE_INTEGER)
    }
  } catch (err) {
    console.error('[EndpointTree.dropToRoot]', err)
  }
}

function startEdit(kind: 'create-folder' | 'rename-folder' | 'rename-endpoint', opts?: { id?: string; parentId?: string | null }): void {
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
}

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
  const name = store.folders.find((f) => f.id === id)?.name ?? ''
  if (!window.confirm(`删除文件夹「${name}」及其全部子文件夹/接口？`)) return
  try {
    await store.deleteFolder(id)
  } catch (err) {
    console.error('[EndpointTree.removeFolder]', err)
  }
}

async function removeEndpoint(e: Endpoint): Promise<void> {
  if (!window.confirm(`删除接口「${e.name || e.path}」？`)) return
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
</script>

<template>
  <div class="tree">
    <template v-for="f in childFolders" :key="f.id">
      <div
        class="tree-row folder-row"
        draggable="true"
        @dragstart="onDragStart($event, 'folder', f.id)"
        @dragover.prevent
        @drop="onDropIntoFolder($event, f.id)"
      >
        <span class="tree-chevron" @click="toggleFolder(f.id)">
          {{ expanded.has(f.id) ? '▾' : '▸' }}
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
          <span class="tree-name folder" @click="toggleFolder(f.id)">📁 {{ f.name }}</span>
          <span class="tree-actions">
            <button type="button" class="tree-btn" title="新建子文件夹" @click="startEdit('create-folder', { parentId: f.id })">＋子夹</button>
            <button type="button" class="tree-btn" title="新建接口" @click="store.openNewEndpoint(f.id)">＋接口</button>
            <button type="button" class="tree-btn" title="导入 cURL" @click="$emit('importCurl', f.id)">cURL</button>
            <button type="button" class="tree-btn" title="重命名" @click="startEdit('rename-folder', { id: f.id })">✎</button>
            <button type="button" class="tree-btn danger" title="删除" @click="removeFolder(f.id)">✕</button>
          </span>
        </template>
      </div>
      <div v-show="expanded.has(f.id)" class="tree-children">
        <EndpointTree :folder-id="f.id" @import-curl="$emit('importCurl', $event)" />
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
        class="tree-row endpoint-row"
        :class="{ active: store.activeTabId === e.id }"
        draggable="true"
        @dragstart="onDragStart($event, 'endpoint', e.id)"
        @dragover.prevent
        @drop="onDropBeforeEndpoint($event, e.id, i)"
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
          <span class="tree-method" :class="`m-${e.method.toLowerCase()}`">{{ e.method }}</span>
          <span class="tree-name" :class="{ dirty: store.isDirty(e.id) }" @click="store.openEndpoint(e)">
            {{ e.name || e.path }}
          </span>
          <span class="tree-actions">
            <button type="button" class="tree-btn" title="复制" @click="duplicate(e)">⧉</button>
            <button type="button" class="tree-btn" title="重命名" @click="startEdit('rename-endpoint', { id: e.id })">✎</button>
            <button type="button" class="tree-btn danger" title="删除" @click="removeEndpoint(e)">✕</button>
          </span>
        </template>
      </div>
    </template>

    <div
      v-if="folderId === null"
      class="tree-roots"
      @dragover.prevent
      @drop="onDropToRoot"
    >
      <button type="button" class="tree-btn root" @click="startEdit('create-folder', { parentId: null })">＋ 新建文件夹</button>
      <button type="button" class="tree-btn root" @click="store.openNewEndpoint(null)">＋ 新建接口</button>
      <button type="button" class="tree-btn root" @click="$emit('importCurl', null)">⇪ 导入 cURL</button>
    </div>
  </div>
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
  padding: 3px 6px;
  border-radius: 6px;
  cursor: default;
}

.tree-row:hover {
  background: rgba(255, 255, 255, 0.05);
}

.tree-row.endpoint-row.active {
  background: rgba(59, 130, 246, 0.14);
}

.tree-chevron {
  width: 14px;
  text-align: center;
  color: var(--rf-text-muted, #6b7280);
  cursor: pointer;
  user-select: none;
}

.tree-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12.5px;
  color: var(--rf-text, #f9fafb);
  cursor: pointer;
}

.tree-name.folder {
  font-weight: 600;
}

.tree-name.dirty::after {
  content: ' ●';
  color: #f59e0b;
}

.tree-method {
  font-size: 10px;
  font-weight: 700;
  padding: 1px 5px;
  border-radius: 4px;
  min-width: 44px;
  text-align: center;
}

.m-get { background: rgba(16, 185, 129, 0.15); color: #34d399; }
.m-post { background: rgba(245, 158, 11, 0.15); color: #fbbf24; }
.m-put { background: rgba(59, 130, 246, 0.15); color: #93c5fd; }
.m-delete { background: rgba(239, 68, 68, 0.15); color: #fca5a5; }
.m-patch { background: rgba(139, 92, 246, 0.15); color: #c4b5fd; }
.m-head, .m-options { background: rgba(107, 114, 128, 0.15); color: #9ca3af; }

.tree-actions {
  display: none;
  gap: 2px;
}

.tree-row:hover .tree-actions {
  display: inline-flex;
}

.tree-btn {
  border: none;
  background: none;
  color: var(--rf-text-secondary, #9ca3af);
  font-size: 11px;
  padding: 1px 4px;
  border-radius: 4px;
  cursor: pointer;
}

.tree-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  color: var(--rf-text, #f9fafb);
}

.tree-btn.danger:hover {
  color: #fca5a5;
}

.tree-btn.root {
  color: var(--rf-text-secondary, #9ca3af);
  text-align: left;
}

.tree-roots {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid var(--rf-border, #1f2937);
}

.tree-input {
  flex: 1;
  min-width: 0;
}

.tree-children {
  padding-left: 14px;
}
</style>
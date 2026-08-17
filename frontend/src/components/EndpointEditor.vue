<script setup lang="ts">
/**
 * EndpointEditor：接口编辑器（草稿模式）。
 *
 * - 直接编辑 store 草稿对象（Map 值经 Vue 集合响应式代理，嵌套修改即跟踪）；
 * - Base URL 为本地临时值（不落库），发送时与 path 拼接；
 * - 配置区为横向 Tab 系统：Params / Auth / Headers / Body / Scripts / Tests / Code，
 *   各渲染独立面板组件（Tests 断言、Code 生成代码已从底部工具区迁入）；
 * - Ctrl+S 保存 / Ctrl+Enter 发送；响应区展示状态码、耗时与正文（JSON 自动美化）。
 */
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useToast } from '../composables/useToast'
import { useFoxApi } from '../composables/useFoxApi'
import { envColorClass, resolveVariables } from '../utils/environment'
import { PROTOCOLS, protocolFromDomain, stripProtocol, withProtocol } from '../utils/url'
import type { Protocol } from '../utils/url'
import AuthPanel from './AuthPanel.vue'
import BodyPanel from './BodyPanel.vue'
import CodeExportMenu from './CodeExportMenu.vue'
import CodeImportDialog from './CodeImportDialog.vue'
import CodePanel from './CodePanel.vue'
import EnvironmentManager from './EnvironmentManager.vue'
import HeadersPanel from './HeadersPanel.vue'
import CustomSelect from './ui/CustomSelect.vue'
import EmptyState from './ui/EmptyState.vue'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import Modal from './ui/Modal.vue'
import ParamsPanel from './ParamsPanel.vue'
import Popconfirm from './ui/Popconfirm.vue'
import ResponsePanel from './ResponsePanel.vue'
import ScriptsPanel from './ScriptsPanel.vue'
import Tabs from './ui/Tabs.vue'
import TestsPanel from './TestsPanel.vue'
import Tooltip from './ui/Tooltip.vue'
import ToolsDrawer from './ToolsDrawer.vue'
import type { TabItem } from './ui/Tabs.vue'
import type {
  ExecuteResponse,
  HttpMethod,
  ResponseExample,
} from '../types/foxApi'

const store = useWorkspaceStore()
const toast = useToast()
const api = useFoxApi()

const sending = ref(false)
/** 在途请求的取消标识（非空表示有请求可取消）。 */
const activeRequestId = ref<string | null>(null)
const response = ref<ExecuteResponse | null>(null)
const sendError = ref<string | null>(null)
/** 多语言代码导入弹窗（cURL / Java / Python / JS / Go → 接口草稿）。 */
const showImportDialog = ref(false)

const draft = computed(() => store.activeEndpoint)

const METHODS: HttpMethod[] = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']
const METHOD_OPTIONS = METHODS.map((m) => ({ value: m, label: m }))

// ---------- 配置 Tab 系统 ----------
type ConfigTabKey = 'params' | 'auth' | 'headers' | 'body' | 'scripts' | 'tests' | 'code'

const activeTab = ref<ConfigTabKey>('params')

const configTabs = computed<TabItem[]>(() => {
  const d = draft.value
  if (!d) return []
  const bodyMode = d.request.body.mode
  return [
    { key: 'params', label: 'Params', count: d.request.params.length },
    { key: 'auth', label: 'Auth' },
    { key: 'headers', label: 'Headers', count: d.request.headers.length },
    { key: 'body', label: 'Body', count: bodyMode !== 'none' ? 1 : undefined },
    { key: 'scripts', label: 'Scripts' },
    { key: 'tests', label: 'Tests', count: d.request.tests ? 1 : undefined },
    { key: 'code', label: 'Code' },
  ]
})

function prettyBody(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2)
  } catch {
    return raw
  }
}

function isAbsolutePath(p: string): boolean {
  return p.startsWith('http://') || p.startsWith('https://')
}

/** 地址栏展示前缀（唯一真实数据源）：环境 base_url 变量 > 会话 Base URL。 */
const urlDomain = computed(() => (draft.value ? store.urlDomain : ''))

/** 面包屑：接口所属文件夹名。 */
const folderName = computed(() => {
  if (!draft.value?.folder_id) return ''
  return store.folders.find((f) => f.id === draft.value!.folder_id)?.name ?? ''
})

/** 路径是否为完整绝对 URL（此时不显示前缀 chip，地址栏直接展示全文）。 */
const isAbsPath = computed(() => (draft.value ? isAbsolutePath(draft.value.path) : false))

/** 激活环境名（chip 色点）。 */
const activeEnvName = computed(
  () => store.environments.find((e) => e.id === store.activeEnvId)?.name ?? '',
)

/** 地址栏前缀 chip 文案：环境 base_url 变量的「解析后」实际值或会话 Base URL。 */
const resolvedDomain = computed(() => {
  const src = urlDomain.value
  if (!src) return ''
  const env = store.environments.find((e) => e.id === store.activeEnvId)
  const vars = { ...(store.project?.variables ?? {}), ...(env?.variables ?? {}) }
  return resolveVariables(src, vars)
})

/** chip 变量引用未解析（环境未定义该变量）。 */
const urlUnresolved = computed(
  () => urlDomain.value.startsWith('{{') && resolvedDomain.value === urlDomain.value,
)

/** 基础 URL 标签样式：环境变量已解析 → 主题色标签；未解析 → 警告；会话回退 → 中性。 */
const chipClass = computed(() => {
  if (urlUnresolved.value) return 'warn'
  if (urlDomain.value.startsWith('{{')) return 'env'
  return 'session'
})

/** 点击基础 URL 标签 → 打开环境管理。 */
const showEnvManager = ref(false)

/** chip 悬浮提示：完整 URL 如何拼接。 */
const urlTooltip = computed(() => {
  if (!draft.value || isAbsPath.value) return ''
  const src = urlDomain.value
  if (src.startsWith('{{')) {
    if (urlUnresolved.value) return `${src} 未定义，请求将按字面量发送`
    const env = store.environments.find((e) => e.id === store.activeEnvId)
    return `${src} → ${resolvedDomain.value}${env ? `（来自环境「${env.name}」）` : ''}`
  }
  return `会话 Base URL：${resolvedDomain.value}（未使用环境变量）`
})

/** 协议选择器选项：与 chip 并列显示，如「https://  api-example.com /api/users」。 */
const PROTOCOL_OPTIONS = PROTOCOLS.map((p) => ({ value: p, label: `${p}://` }))

/** 路径输入框元素引用（快捷按钮聚焦回跳）。 */
const urlInputEl = ref<HTMLInputElement | null>(null)

/** 协议选择器：从「解析后的」域名源推导当前 scheme，改写时同步环境 base_url 变量或会话 Base URL。 */
const protocol = computed({
  get: () => protocolFromDomain(resolvedDomain.value || urlDomain.value),
  set: (p: Protocol) => {
    const src = resolvedDomain.value || urlDomain.value
    if (src.trim().startsWith('{{')) return
    const next = withProtocol(src, p)
    if (urlDomain.value.startsWith('{{')) {
      void store.setEnvironmentBaseUrl(next)
    } else {
      store.sessionBaseUrl = next
    }
  },
})

/** chip 展示文案：协议前缀由选择器承担，chip 只显示裸域名。 */
const domainLabel = computed(() => {
  const src = resolvedDomain.value
  if (!src) return ''
  if (src === urlDomain.value) return src
  return stripProtocol(src) || src
})

/** 路径输入框（与 chip 组成完整请求地址）；粘贴完整 URL 时自动拆分。 */
const urlPath = computed({
  get: () => {
    const d = draft.value
    if (!d) return ''
    return d.path
  },
  set: (value: string) => {
    const d = draft.value
    if (!d) return
    const v = value.trim()
    if (!v) return

    // 1) 粘贴/改写完整 URL：origin 写入域名源（环境变量优先），query 并入参数。
    const abs = v.match(/^(?:https?|wss?):\/\/[^/]+/)
    if (abs) {
      let rest = v.slice(abs[0].length) || '/'
      const qIdx = rest.indexOf('?')
      if (qIdx !== -1) {
        const qs = rest.slice(qIdx + 1)
        rest = rest.slice(0, qIdx) || '/'
        for (const [key, val] of new URLSearchParams(qs).entries()) {
          d.request.params.push({ key, value: val, enabled: true, description: '' })
        }
      }
      if (store.urlDomain.startsWith('{{')) {
        void store.setEnvironmentBaseUrl(abs[0])
      } else {
        store.sessionBaseUrl = abs[0]
      }
      d.path = rest.startsWith('/') ? rest : `/${rest}`
      return
    }

    // 2) 以 `{{变量}}` 开头：变量引用成为域名源。
    const varRef = v.match(/^\{\{[^{}]+\}\}/)
    if (varRef) {
      store.sessionBaseUrl = varRef[0]
      d.path = v.slice(varRef[0].length) || '/'
      return
    }

    // 3) 其余视为路径本身。
    d.path = v.startsWith('/') ? v : `/${v}`
  },
})

/** 请求地址（与 send / 代码生成 / 压测共用）；变量由后端按环境注入。 */
function buildUrl(): string {
  const d = draft.value
  if (!d) return ''
  if (isAbsolutePath(d.path)) return d.path
  const path = d.path.startsWith('/') ? d.path : `/${d.path}`
  return `${store.urlDomain}${path}`
}

async function send(): Promise<void> {
  if (!draft.value || sending.value) return
  sending.value = true
  sendError.value = null
  const url = buildUrl()
  const rid = crypto.randomUUID()
  activeRequestId.value = rid
  try {
    response.value = await store.send(draft.value, url, rid)
    // 历史已迁至侧栏「请求历史」页签，发送后由 store 统一刷新。
    void store.loadHistories()
  } catch (err) {
    const e = err as Error & { code?: string }
    if (e?.code === 'CANCELLED') {
      // 用户主动取消：不视为错误，保留上一次结果。
      toast.info('请求已取消')
      sendError.value = null
    } else {
      sendError.value = err instanceof Error ? err.message : String(err)
      response.value = null
    }
  } finally {
    if (activeRequestId.value === rid) activeRequestId.value = null
    sending.value = false
  }
}

/** 取消在途请求（后端中止连接，命令随即以 CANCELLED 返回）。 */
function cancelSend(): void {
  if (!activeRequestId.value) return
  void api.cancelRequest(activeRequestId.value)
  toast.info('正在取消请求…')
}

/** 保存：名称为空时先弹名称输入框，确认后再落库。 */
const showNameDialog = ref(false)
const pendingName = ref('')

async function save(): Promise<void> {
  if (!draft.value) return
  if (!draft.value.name.trim()) {
    pendingName.value = ''
    showNameDialog.value = true
    return
  }
  await store.saveActiveDraft()
}

async function confirmName(): Promise<void> {
  if (!draft.value) return
  const name = pendingName.value.trim()
  if (!name) {
    toast.warning('接口名称不能为空')
    return
  }
  draft.value.name = name
  showNameDialog.value = false
  await store.saveActiveDraft()
}

// ---------- 响应示例 ----------
const viewingExample = ref<ResponseExample | null>(null)
const activeExamples = computed(() => store.examples.get(draft.value?.id ?? '') ?? [])

const showExampleDialog = ref(false)
const exampleName = ref('')

function saveExample(): void {
  if (!draft.value || !response.value) return
  exampleName.value = `${draft.value.method} ${new Date().toLocaleTimeString('zh-CN')}`
  showExampleDialog.value = true
}

async function confirmSaveExample(): Promise<void> {
  if (!draft.value || !response.value) return
  const name = exampleName.value.trim()
  if (!name) {
    toast.warning('示例名称不能为空')
    return
  }
  try {
    await store.saveAsExample(draft.value.id, name, response.value)
    showExampleDialog.value = false
  } catch (err) {
    toast.error('保存示例失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

function viewExample(ex: ResponseExample): void {
  viewingExample.value = viewingExample.value?.id === ex.id ? null : ex
}

async function removeExample(ex: ResponseExample): Promise<void> {
  if (!draft.value) return
  try {
    await store.removeExample(draft.value.id, ex.id)
    if (viewingExample.value?.id === ex.id) viewingExample.value = null
  } catch (err) {
    toast.error('删除示例失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

// ---------- 工具抽屉（生成代码 / 测试 / 压测） ----------
const showTools = ref(false)

const requestUrl = computed(() => (draft.value ? buildUrl() : ''))

/** 路径输入框 Enter → 发送；Esc → 清空路径（setter 忽略空串，直接写草稿）。 */
function onUrlKeydown(event: KeyboardEvent): void {
  if (event.key === 'Enter') {
    event.preventDefault()
    event.stopPropagation()
    if (!sending.value) void send()
  } else if (event.key === 'Escape') {
    event.preventDefault()
    if (draft.value) draft.value.path = ''
  }
}

/** 快捷按钮：清空路径。 */
function clearPath(): void {
  if (draft.value) draft.value.path = ''
}

/** 快捷按钮：复制完整请求地址。 */
async function copyRequestUrl(): Promise<void> {
  if (!requestUrl.value) return
  await navigator.clipboard.writeText(requestUrl.value)
  toast.info('地址已复制')
}

function onKeydown(event: KeyboardEvent): void {
  if (!(event.metaKey || event.ctrlKey)) return
  if (event.key === 's') {
    event.preventDefault()
    save()
  } else if (event.key === 'Enter') {
    event.preventDefault()
    send()
  } else if (event.key === 't') {
    event.preventDefault()
    store.openNewEndpoint(null)
  }
}

/** 新建接口后自动聚焦标题并全选，便于直接输入名称（TabBar「+」/ ⌘T / 树内新建共用）。 */
const crumbNameInput = ref<HTMLInputElement | null>(null)
watch(
  () => store.focusTitleSignal,
  () => {
    void nextTick(() => {
      crumbNameInput.value?.focus()
      crumbNameInput.value?.select()
    })
  },
)

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <div v-if="draft" class="editor">
    <div class="editor-row breadcrumb-row">
      <span class="crumb">
        <span class="crumb-part">{{ store.project?.name ?? '未命名项目' }}</span>
        <template v-if="folderName">
          <span class="crumb-sep">/</span>
          <span class="crumb-part">{{ folderName }}</span>
        </template>
        <span class="crumb-sep">/</span>
        <input
          ref="crumbNameInput"
          v-model="draft.name"
          class="crumb-name"
          placeholder="接口名称"
          spellcheck="false"
          title="点击可直接修改接口名称"
        />
      </span>
      <span class="breadcrumb-spacer"></span>
    </div>
    <div class="editor-row">
      <div class="request-bar">
        <CustomSelect
          class="method-select"
          :model-value="draft.method"
          :options="METHOD_OPTIONS"
          @update:model-value="draft.method = String($event) as HttpMethod"
        >
          <template #display="{ label }">
            <span :class="`m-select-${draft.method.toLowerCase()}`">{{ label }}</span>
          </template>
        </CustomSelect>
        <span class="req-bar-divider"></span>
        <CustomSelect
          v-if="urlDomain && !isAbsPath"
          class="protocol-select"
          :model-value="protocol"
          :options="PROTOCOL_OPTIONS"
          size="sm"
          @update:model-value="protocol = String($event) as Protocol"
        />
        <Tooltip v-if="urlDomain && !isAbsPath" :content="urlTooltip" placement="bottom">
          <button
            type="button"
            class="url-chip"
            :class="chipClass"
            title="点击管理环境"
            @click="showEnvManager = true"
          >
            <span v-if="activeEnvName" class="edot" :class="`ed-${envColorClass(activeEnvName)}`"></span>
            <Icon name="globe" :size="13" class="url-chip-icon" />
            <span class="url-chip-text">{{ domainLabel }}</span>
            <Icon name="chevron-down" :size="12" class="url-chip-chevron" />
          </button>
        </Tooltip>
        <span v-if="urlDomain && !isAbsPath" class="req-bar-divider"></span>
        <div class="url-input-wrap">
          <input
            ref="urlInputEl"
            v-model="urlPath"
            class="url-input"
            spellcheck="false"
            placeholder="路径，如 /api/users；可粘贴完整 URL 自动拆分"
            @keydown="onUrlKeydown"
          />
          <template v-if="urlPath">
            <button
              type="button"
              class="url-qbtn url-qbtn-copy"
              title="复制完整请求地址"
              @click="copyRequestUrl"
            >
              <Icon name="copy" :size="13" />
            </button>
            <button
              type="button"
              class="url-qbtn url-qbtn-clear"
              title="清空路径 (Esc)"
              @click="clearPath"
            >
              <Icon name="x" :size="13" />
            </button>
          </template>
        </div>
      </div>
      <div class="editor-actions">
        <button class="rf-btn rf-btn-sm" type="button" title="压测（并发基准）" @click="showTools = true">
          <Icon name="gauge" :size="13" /> 工具
        </button>
        <button v-if="!sending" class="rf-btn rf-btn-send" type="button" @click="send">
          <Icon name="send" :size="14" />
          发送 (⌘⏎)
        </button>
        <button
          v-else
          class="rf-btn rf-btn-danger"
          type="button"
          title="取消在途请求"
          @click="cancelSend"
        >
          <Icon name="stop" :size="14" /> 取消请求
        </button>
        <CodeExportMenu :draft="draft" :url="requestUrl" />
        <button class="rf-btn" type="button" @click="save">
          <Icon name="save" :size="14" /> 保存 (⌘S)
        </button>
        <button
          class="rf-btn"
          type="button"
          title="从 cURL / Java / Python / JavaScript / Go 代码导入为新接口"
          @click="showImportDialog = true"
        >
          <Icon name="download" :size="14" /> 导入
        </button>
      </div>
    </div>

    <div class="config-box">
      <Tabs v-model="activeTab" :tabs="configTabs" size="sm" />
      <ParamsPanel v-if="activeTab === 'params'" :draft="draft" />
      <AuthPanel v-else-if="activeTab === 'auth'" :draft="draft" />
      <HeadersPanel v-else-if="activeTab === 'headers'" :draft="draft" />
      <BodyPanel v-else-if="activeTab === 'body'" :draft="draft" />
      <ScriptsPanel v-else-if="activeTab === 'scripts'" :draft="draft" />
      <TestsPanel v-else-if="activeTab === 'tests'" :draft="draft" :url="requestUrl" />
      <CodePanel v-else :draft="draft" :url="requestUrl" />
    </div>

    <div class="response-zone">
      <ResponsePanel v-if="response" :response="response" @save-example="saveExample" />
      <div v-else-if="sendError" class="send-error" role="alert">
        <span>发送失败：{{ sendError }}</span>
      </div>
      <EmptyState
        v-else
        class="response-empty"
        icon="send"
        title="尚未发送请求"
        description="点击发送按钮或按 Cmd + Enter (Ctrl + Enter) 获取响应结果"
      />
    </div>
    <div v-if="activeExamples.length" class="examples">
      <h3 class="section-title">响应示例 ({{ activeExamples.length }})</h3>
      <div v-for="ex in activeExamples" :key="ex.id" class="example-row">
        <button
          class="example-main"
          type="button"
          :class="{ open: viewingExample?.id === ex.id }"
          @click="viewExample(ex)"
        >
          <span class="example-status" :class="{ err: ex.status >= 400 }">{{ ex.status }}</span>
          <span class="example-name">{{ ex.name }}</span>
          <span class="example-meta">{{ ex.created_at.slice(0, 16).replace('T', ' ') }}</span>
        </button>
        <Popconfirm :title="`删除示例「${ex.name}」？`" @confirm="removeExample(ex)">
            <IconButton name="trash" :size="13" tone="danger" title="删除示例" />
          </Popconfirm>
      </div>
      <pre v-if="viewingExample" class="example-body">{{ prettyBody(viewingExample.body) }}</pre>
    </div>
  </div>
  <div v-else class="editor-empty">
    <p>从左侧选择接口开始编辑</p>
  </div>

  <Modal v-model:open="showNameDialog" title="保存接口" width="360px">
    <p class="name-hint">请为接口填写一个名称（必填）：</p>
    <input
      v-model="pendingName"
      class="rf-input name-dialog-input"
      placeholder="例如：获取用户列表"
      spellcheck="false"
      @keyup.enter="confirmName"
    />
    <template #footer>
      <button class="rf-btn" type="button" @click="showNameDialog = false">取消</button>
      <button class="rf-btn rf-btn-primary" type="button" @click="confirmName">
        <Icon name="save" :size="14" /> 保存
      </button>
    </template>
  </Modal>

  <Modal v-model:open="showExampleDialog" title="保存响应示例" width="360px">
    <p class="name-hint">请输入示例名称：</p>
    <input
      v-model="exampleName"
      class="rf-input name-dialog-input"
      placeholder="例如：成功响应"
      spellcheck="false"
      @keyup.enter="confirmSaveExample"
    />
    <template #footer>
      <button class="rf-btn" type="button" @click="showExampleDialog = false">取消</button>
      <button class="rf-btn rf-btn-primary" type="button" @click="confirmSaveExample">保存</button>
    </template>
  </Modal>

  <ToolsDrawer :open="showTools" :draft="draft" :url="requestUrl" @close="showTools = false" />

  <EnvironmentManager v-model:open="showEnvManager" />

  <CodeImportDialog
    v-if="showImportDialog"
    :folder-id="draft?.folder_id ?? null"
    @close="showImportDialog = false"
  />
</template>

<style scoped>
.editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  overflow-y: auto;
  height: 100%;
}

.editor-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.method-select {
  width: 108px;
  flex-shrink: 0;
  font-weight: 700;
}

.m-select-get { color: var(--rf-success); }
.m-select-post { color: var(--rf-warning); }
.m-select-put { color: var(--rf-info); }
.m-select-delete { color: var(--rf-danger); }
.m-select-patch { color: var(--patch); }
.m-select-head, .m-select-options { color: var(--rf-text-muted); }

/* 统一请求栏：方法下拉 + 基础URL标签 + 路径输入合并为一个控件 */
.request-bar {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: stretch;
  height: var(--h-md);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-card);
  overflow: hidden;
  transition:
    border-color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.request-bar:hover {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent-tint);
}
.request-bar:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint);
}

.request-bar .method-select {
  width: 116px;
  border: none;
  background: var(--bg-panel);
}
.request-bar .method-select :deep(.cs-trigger) {
  height: 100%;
  border: none;
  background: transparent;
  box-shadow: none;
  border-radius: 0;
}

.req-bar-divider {
  width: 1px;
  flex-shrink: 0;
  background: var(--border);
}

/* Tooltip 触发包裹层需允许收缩，避免挤压路径输入 */
.request-bar :deep(.tt-trigger) {
  min-width: 0;
}

/* 基础 URL 标签：圆角胶囊，可点击打开环境管理 */
.url-chip {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 28px;
  margin: 2px 4px;
  padding: 0 12px;
  border: 1px solid transparent;
  border-radius: 999px;
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 600;
  line-height: 1;
  white-space: nowrap;
  max-width: 360px;
  cursor: pointer;
  transition:
    background var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease),
    filter var(--dur) var(--ease);
}
.url-chip:hover {
  box-shadow: 0 0 0 2px var(--accent-tint);
}

/* 环境 base_url 变量已解析：主题色（蓝/紫）标签 */
.url-chip.env {
  background: var(--accent-tint);
  color: var(--accent);
}
.url-chip.env:hover {
  filter: brightness(0.96);
}

/* 变量未定义（将按字面量发送）：警告色标签 */
.url-chip.warn {
  background: var(--warning-tint);
  color: var(--warning);
}
.url-chip.warn:hover {
  filter: brightness(0.96);
}

/* 会话级 Base URL（未使用环境变量）：中性标签 */
.url-chip.session {
  background: var(--bg-hover);
  color: var(--text-2);
  border-color: var(--border);
}

.url-chip-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.url-chip-icon {
  flex-shrink: 0;
}

.url-chip-chevron {
  flex-shrink: 0;
  opacity: 0.55;
  transition: opacity var(--dur) var(--ease);
}
.url-chip:hover .url-chip-chevron {
  opacity: 1;
}

/* 协议选择器：与 chip 同视觉层级（无边框透明触发区，宽度固定） */
.request-bar .protocol-select {
  flex-shrink: 0;
  width: 88px;
}
.request-bar .protocol-select :deep(.cs-trigger) {
  height: 100%;
  border: none;
  background: transparent;
  box-shadow: none;
  border-radius: 0;
  font-family: var(--font-mono);
  justify-content: center;
}

/* 环境色点 */
.edot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--text-3);
}
.ed-dev {
  background: var(--success);
}
.ed-test {
  background: var(--info);
}
.ed-staging {
  background: var(--warning);
}
.ed-prod {
  background: #f97316;
}
.ed-global {
  background: var(--accent);
}

.request-bar .url-input-wrap {
  position: relative;
  display: flex;
  align-items: center;
  flex: 1;
  min-width: 0;
  height: 100%;
}

.request-bar .url-input {
  flex: 1;
  min-width: 0;
  height: 100%;
  border: none;
  background: transparent;
  box-shadow: none;
  border-radius: 0;
  padding: 0 62px 0 10px;
  font-family: var(--font-mono);
}

/* 地址栏快捷按钮：悬停输入框时浮现（手动 drop-shadow 模拟浅分隔） */
.url-qbtn {
  position: absolute;
  top: 50%;
  right: 6px;
  transform: translateY(-50%);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-2);
  cursor: pointer;
  transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
}
.url-qbtn-copy {
  right: 32px;
}
.url-qbtn:hover {
  background: var(--bg-hover);
  color: var(--text-1);
}

/* ---- 面包屑行（接口名称移至此处） ---- */
.breadcrumb-row {
  gap: 8px;
}

.crumb {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  flex: 1;
  font-size: var(--fs-sm);
}

.crumb-part {
  color: var(--text-2);
  white-space: nowrap;
}

.crumb-sep {
  color: var(--text-3);
}

/* 接口标题：内联编辑样式——常态为面包屑文本，hover 显示虚线提示可编辑，聚焦高亮 */
.crumb-name {
  min-width: 60px;
  max-width: 280px;
  font-size: var(--fs-sm);
  font-weight: 600;
  color: var(--text-1);
  background: transparent;
  border: none;
  border-bottom: 1px dashed transparent;
  border-radius: 0;
  padding: 1px 2px;
  cursor: text;
  transition: border-bottom-color var(--dur) var(--ease);
}

.crumb-name:hover {
  border-bottom-color: var(--text-3);
  background: transparent;
}

.crumb-name:focus {
  outline: none;
  border-bottom-color: var(--accent);
  background: transparent;
}

.breadcrumb-spacer {
  flex: 1;
}

.editor-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.config-box {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.kv-remove {
  color: var(--rf-text-muted);
}

.body-mode-select {
  width: 200px;
}

.mp-type {
  width: 110px;
  flex-shrink: 0;
}

.oauth-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.oauth-hint {
  margin: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12px;
  color: var(--rf-text-muted);
}

.oauth-status.ok {
  color: var(--rf-success);
}

.response-save {
  margin-left: auto;
}

.examples {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.example-row {
  display: flex;
  align-items: center;
  gap: 6px;
}

.example-main {
  flex: 1;
  display: flex;
  align-items: center;
  gap: 8px;
  border: 1px solid var(--rf-border);
  background: var(--rf-input-bg);
  border-radius: 6px;
  padding: 5px 10px;
  cursor: pointer;
  color: var(--rf-text, #f9fafb);
  font-size: 12.5px;
  text-align: left;
}

.example-main.open {
  border-color: var(--rf-info);
}

.example-status {
  font-weight: 700;
  font-size: 11px;
  color: var(--rf-success);
}

.example-status.err {
  color: var(--rf-danger);
}

.example-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.example-meta {
  font-size: 11px;
  color: var(--rf-text-muted);
}

.example-body {
  margin: 0;
  padding: 10px 12px;
  background: var(--rf-input-bg);
  border: 1px solid var(--rf-border);
  border-radius: 6px;
  font-family: var(--font-mono);
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 240px;
  overflow-y: auto;
}

.send-error {
  padding: 10px 12px;
  border-radius: var(--radius);
  background: var(--danger-tint);
  border: 1px solid var(--danger-border);
  color: var(--danger);
  font-size: 12.5px;
}

/* ---- 响应容器：有响应时由 ResponsePanel 填充，未发送时显示空态 ---- */
.response-zone {
  display: flex;
  flex-direction: column;
}

.response-empty {
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-card);
}

.editor-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-3);
}

/* ---- 名称输入对话框 ---- */

.name-hint {
  margin: 0 0 8px;
  font-size: 12.5px;
  color: var(--text-2);
}

.name-dialog-input {
  width: 100%;
  height: var(--h-md);
}
</style>
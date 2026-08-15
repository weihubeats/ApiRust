<script setup lang="ts">
/**
 * EndpointEditor：接口编辑器（草稿模式）。
 *
 * - 直接编辑 store 草稿对象（Map 值经 Vue 集合响应式代理，嵌套修改即跟踪）；
 * - Base URL 为本地临时值（不落库），发送时与 path 拼接；
 * - Ctrl+S 保存 / Ctrl+Enter 发送；响应区展示状态码、耗时与正文（JSON 自动美化）。
 */
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useWorkspaceStore } from '../stores/workspace'
import { useToast } from '../composables/useToast'
import { useFoxApi } from '../composables/useFoxApi'
import type {
  CodeLang,
  EndpointResult,
  ExecuteResponse,
  HttpMethod,
  KeyValue,
  LoadResult,
  MultipartField,
  RequestHistory,
  ResponseExample,
} from '../types/foxApi'

const store = useWorkspaceStore()
const toast = useToast()
const api = useFoxApi()

const baseUrl = ref('http://localhost')
const sending = ref(false)
const response = ref<ExecuteResponse | null>(null)
const sendError = ref<string | null>(null)

const draft = computed(() => store.activeEndpoint)

/** Body 编辑区只支持 none/json/text/graphql；urlencoded/multipart 后续阶段接入。
 *  bodyAny 用 any 放宽联合类型访问（模板 v-model 直写 raw / spec.*）。 */
const bodyAny = computed(() => draft.value?.request.body as any)
const graphql = computed(() => bodyAny.value?.spec as any)
const METHODS: HttpMethod[] = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']
const BODY_MODES: Array<{ value: string; label: string }> = [
  { value: 'none', label: '无 Body' },
  { value: 'json', label: 'JSON' },
  { value: 'text', label: 'Text' },
  { value: 'graphql', label: 'GraphQL' },
  { value: 'urlencoded', label: '表单 (x-www-form-urlencoded)' },
  { value: 'multipart', label: '多部件 (multipart/form-data)' },
]

/** Body 模式切换：整体替换为对应形状的默认对象（避免残留多余字段）。 */
function setBodyMode(mode: string): void {
  if (!draft.value) return
  const prev = bodyAny.value
  switch (mode) {
    case 'none':
      draft.value.request.body = { mode: 'none' }
      break
    case 'json':
    case 'text':
      draft.value.request.body = { mode, raw: prev?.raw ?? '' }
      break
    case 'graphql':
      draft.value.request.body = {
        mode: 'graphql',
        spec: { query: prev?.spec?.query ?? '', variables: prev?.spec?.variables ?? '{}', operation_name: prev?.spec?.operation_name ?? '' },
      }
      break
    case 'urlencoded':
      draft.value.request.body = { mode: 'urlencoded', fields: prev?.fields ?? [] }
      break
    case 'multipart':
      draft.value.request.body = { mode: 'multipart', fields: prev?.fields ?? [] }
      break
    default:
      draft.value.request.body = { mode: 'none' }
  }
}

const AUTH_TYPES: Array<{ value: string; label: string }> = [
  { value: 'none', label: '无认证' },
  { value: 'bearer', label: 'Bearer Token' },
  { value: 'basic', label: 'Basic' },
  { value: 'apikey', label: 'API Key' },
  { value: 'oauth2', label: 'OAuth2' },
]

/** Auth 编辑区；type 切换时替换为对应默认对象。 */
const authAny = computed(() => draft.value?.request.auth as any)
const authorizing = ref(false)

/** OAuth2 授权状态文案。 */
const oauthStatus = computed(() => {
  const token = authAny.value?.token as
    | { access_token?: string; expires_at?: string }
    | undefined
  if (!token?.access_token) return '未授权'
  const expires = token.expires_at ? new Date(token.expires_at) : null
  const expiring = expires ? expires.getTime() - Date.now() < 5 * 60_000 : false
  return expires && !expiring
    ? `已授权，有效期至 ${expires.toLocaleString('zh-CN')}`
    : expiring
      ? '令牌即将过期，发送时将自动刷新'
      : '已授权（发送时自动刷新）'
})

/** 发起完整授权流：后端起本地回调 + 打开系统浏览器；完成后令牌写入草稿。 */
async function oauthAuthorize(): Promise<void> {
  if (!draft.value) return
  authorizing.value = true
  try {
    const token = await api.oauthAuthorize(authAny.value)
    draft.value.request.auth = { ...authAny.value, token }
    toast.success('OAuth2 授权成功，请保存 (⌘S) 持久化')
  } catch (err) {
    toast.error('OAuth2 授权失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    authorizing.value = false
  }
}

function setAuthType(type: string): void {
  if (!draft.value) return
  switch (type) {
    case 'none':
      draft.value.request.auth = { type: 'none' }
      break
    case 'bearer':
      draft.value.request.auth = { type: 'bearer', token: '' }
      break
    case 'basic':
      draft.value.request.auth = { type: 'basic', username: '', password: '' }
      break
    case 'apikey':
      draft.value.request.auth = { type: 'apikey', key: '', value: '', in: 'header' }
      break
    case 'oauth2':
      draft.value.request.auth = {
        type: 'oauth2',
        client_id: '',
        client_secret: '',
        auth_url: '',
        token_url: '',
        scope: '',
        redirect_uri: '',
      }
      break
  }
}

function addUrlencodedField(): void {
  const fields = draft.value?.request.body as { fields: KeyValue[] } | undefined
  fields?.fields.push({ key: '', value: '', enabled: true, description: '' })
}

function removeUrlencodedField(index: number): void {
  const fields = draft.value?.request.body as { fields: unknown[] } | undefined
  fields?.fields.splice(index, 1)
}

function addMultipartField(): void {
  const fields = draft.value?.request.body as { fields: MultipartField[] } | undefined
  fields?.fields.push({ key: '', value_type: 'text', value: '', enabled: true })
}

function removeMultipartField(index: number): void {
  const fields = draft.value?.request.body as { fields: unknown[] } | undefined
  fields?.fields.splice(index, 1)
}

function addParam(): void {
  draft.value?.request.params.push({ key: '', value: '', enabled: true, description: '' })
}

function removeParam(index: number): void {
  draft.value?.request.params.splice(index, 1)
}

function addHeader(): void {
  draft.value?.request.headers.push({ key: '', value: '', enabled: true, description: '' })
}

function removeHeader(index: number): void {
  draft.value?.request.headers.splice(index, 1)
}

function prettyBody(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2)
  } catch {
    return raw
  }
}

/** 拼接请求地址（与 send / 代码生成共用）。 */
function buildUrl(): string {
  if (!draft.value) return ''
  return draft.value.path.startsWith('http://') || draft.value.path.startsWith('https://')
    ? draft.value.path
    : `${baseUrl.value.replace(/\/+$/, '')}${draft.value.path}`
}

async function send(): Promise<void> {
  if (!draft.value) return
  sending.value = true
  sendError.value = null
  const url = buildUrl()
  try {
    response.value = await store.send(draft.value, url)
    loadHistory()
  } catch (err) {
    sendError.value = err instanceof Error ? err.message : String(err)
    response.value = null
  } finally {
    sending.value = false
  }
}

async function save(): Promise<void> {
  await store.saveActiveDraft()
}

async function newEnvironment(): Promise<void> {
  const name = window.prompt('环境名称')
  if (!name?.trim()) return
  try {
    await store.createEnvironment(name.trim())
  } catch (err) {
    toast.error('创建环境失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

// ---------- 响应示例 ----------
const viewingExample = ref<ResponseExample | null>(null)
const activeExamples = computed(() => store.examples.get(draft.value?.id ?? '') ?? [])

async function saveExample(): Promise<void> {
  if (!draft.value || !response.value) return
  const name = window.prompt('示例名称', `${draft.value.method} ${new Date().toLocaleTimeString('zh-CN')}`)
  if (!name?.trim()) return
  try {
    await store.saveAsExample(draft.value.id, name.trim(), response.value)
  } catch (err) {
    toast.error('保存示例失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

function viewExample(ex: ResponseExample): void {
  viewingExample.value = viewingExample.value?.id === ex.id ? null : ex
}

async function removeExample(ex: ResponseExample): Promise<void> {
  if (!draft.value) return
  if (!window.confirm(`删除示例「${ex.name}」？`)) return
  try {
    await store.removeExample(draft.value.id, ex.id)
    if (viewingExample.value?.id === ex.id) viewingExample.value = null
  } catch (err) {
    toast.error('删除示例失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

// ---------- 代码生成 ----------
const CODE_LANGS: Array<{ value: CodeLang; label: string }> = [
  { value: 'curl', label: 'cURL' },
  { value: 'python', label: 'Python (requests)' },
  { value: 'js', label: 'JavaScript (fetch)' },
  { value: 'go', label: 'Go (net/http)' },
  { value: 'java', label: 'Java (OkHttp)' },
  { value: 'php', label: 'PHP (cURL)' },
]
const codeLang = ref<CodeLang>('curl')
const generatedCode = ref<string | null>(null)
const generating = ref(false)

async function generateCode(): Promise<void> {
  if (!draft.value) return
  generating.value = true
  try {
    generatedCode.value = await api.codegenRender({
      lang: codeLang.value,
      method: draft.value.method,
      url: buildUrl(),
      headers: draft.value.request.headers,
      body: draft.value.request.body,
      auth: draft.value.request.auth,
    })
  } catch (err) {
    toast.error('生成代码失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    generating.value = false
  }
}

async function copyCode(): Promise<void> {
  if (!generatedCode.value) return
  try {
    await navigator.clipboard.writeText(generatedCode.value)
    toast.success('已复制到剪贴板')
  } catch {
    toast.error('复制失败，请手动选择文本')
  }
}

// ---------- 请求历史 ----------
const histories = ref<RequestHistory[]>([])

async function loadHistory(): Promise<void> {
  if (!store.project) return
  try {
    histories.value = (await api.listRequestHistories(store.project.id, 30)) ?? []
  } catch {
    // 历史查询失败不打扰编辑流程
  }
}

function historySummary(h: RequestHistory): { status: number; size: number } | null {
  try {
    const data = JSON.parse(h.response_summary_json) as { status?: number; size_bytes?: number }
    return data.status != null ? { status: data.status, size: data.size_bytes ?? 0 } : null
  } catch {
    return null
  }
}

// ---------- 测试 / 压测 ----------
const testsJson = ref('')
const testResult = ref<EndpointResult | null>(null)
const testing = ref(false)

watch(
  draft,
  () => {
    const tests = (draft.value?.request as any)?.tests
    testsJson.value = tests ? JSON.stringify(tests, null, 2) : ''
  },
  { deep: true, immediate: true },
)

async function runTests(): Promise<void> {
  if (!draft.value) return
  try {
    ;(draft.value.request as any).tests = testsJson.value.trim()
      ? JSON.parse(testsJson.value)
      : null
  } catch {
    toast.error('测试配置不是合法 JSON')
    return
  }
  testing.value = true
  try {
    testResult.value = await api.testEndpoint({
      endpoint: draft.value,
      url: buildUrl(),
      environment_id: store.activeEnvId,
    })
  } catch (err) {
    toast.error('测试运行失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    testing.value = false
  }
}

const loadConcurrency = ref('20')
const loadTotal = ref('200')
const loadResult = ref<LoadResult | null>(null)
const loading = ref(false)

async function runLoadTest(): Promise<void> {
  if (!draft.value) return
  const concurrency = Number(loadConcurrency.value)
  const total = Number(loadTotal.value)
  if (!Number.isFinite(concurrency) || !Number.isFinite(total) || total < 1) {
    toast.error('请输入合法的并发数与总数')
    return
  }
  loading.value = true
  loadProgress.value = null
  loadResult.value = null
  try {
    loadResult.value = await api.loadTest({
      url: buildUrl(),
      method: draft.value.method,
      spec: draft.value.request,
      environment_id: store.activeEnvId,
      concurrency,
      total,
    })
  } catch (err) {
    toast.error('压测失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    loading.value = false
    loadProgress.value = null
  }
}

function onKeydown(event: KeyboardEvent): void {
  if (!(event.metaKey || event.ctrlKey)) return
  if (event.key === 's') {
    event.preventDefault()
    save()
  } else if (event.key === 'Enter') {
    event.preventDefault()
    send()
  }
}

const loadProgress = ref<{ done: number; total: number; ok: number; failed: number } | null>(null)
let unlistenLoad: UnlistenFn | null = null

onMounted(async () => {
  window.addEventListener('keydown', onKeydown)
  loadHistory()
  unlistenLoad = await listen<{ done: number; total: number; ok: number; failed: number }>(
    'fox:load-progress',
    (event) => {
      loadProgress.value = event.payload
    },
  )
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
  unlistenLoad?.()
})
</script>

<template>
  <div v-if="draft" class="editor">
    <div class="editor-row">
      <select v-model="draft.method" class="rf-input method-select" :class="`m-select-${draft.method.toLowerCase()}`">
        <option v-for="m in METHODS" :key="m" :value="m">{{ m }}</option>
      </select>
      <input v-model="draft.path" class="rf-input path-input" spellcheck="false" placeholder="/api/users" />
      <div class="editor-actions">
        <button class="rf-btn rf-btn-primary" type="button" :disabled="sending" @click="send">
          {{ sending ? '发送中…' : '发送 (⌘⏎)' }}
        </button>
        <button class="rf-btn" type="button" @click="save">保存 (⌘S)</button>
      </div>
    </div>
    <div class="editor-row">
      <input v-model="draft.name" class="rf-input name-input" placeholder="接口名称（必填）" />
      <select
        :value="store.activeEnvId ?? ''"
        class="rf-input rf-input-sm env-select"
        @change="store.setEnvironment(($event.target as HTMLSelectElement).value || null)"
      >
        <option value="">环境：无</option>
        <option v-for="env in store.environments" :key="env.id" :value="env.id">
          {{ env.name }}
        </option>
      </select>
      <button class="rf-btn rf-btn-sm env-new" type="button" title="新建环境" @click="newEnvironment">
        ＋
      </button>
      <input
        v-model="baseUrl"
        class="rf-input base-input"
        spellcheck="false"
        placeholder="Base URL（仅本次会话，不落库）"
      />
    </div>

    <div class="editor-section">
      <h3 class="section-title">查询参数 (Params)</h3>
      <div v-for="(p, i) in draft.request.params" :key="i" class="kv-row">
        <input v-model="p.enabled" type="checkbox" class="kv-check" />
        <input v-model="p.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
        <input v-model="p.value" class="rf-input rf-input-sm kv-value" placeholder="Value" />
        <button class="rf-btn rf-btn-sm kv-remove" type="button" @click="removeParam(i)">✕</button>
      </div>
      <button class="rf-btn rf-btn-sm" type="button" @click="addParam">＋ 添加参数</button>
    </div>

    <div class="editor-section">
      <h3 class="section-title">认证 (Auth)</h3>
      <select
        :value="authAny?.type ?? 'none'"
        class="rf-input rf-input-sm auth-type-select"
        @change="setAuthType(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="t in AUTH_TYPES" :key="t.value" :value="t.value">{{ t.label }}</option>
      </select>
      <div v-if="authAny?.type === 'bearer'" class="kv-row">
        <input
          v-model="authAny.token"
          class="rf-input rf-input-sm kv-value"
          placeholder="Token"
          spellcheck="false"
        />
      </div>
      <div v-else-if="authAny?.type === 'basic'" class="kv-row">
        <input
          v-model="authAny.username"
          class="rf-input rf-input-sm kv-key"
          placeholder="用户名"
        />
        <input
          v-model="authAny.password"
          class="rf-input rf-input-sm kv-value"
          placeholder="密码"
          type="password"
        />
      </div>
      <div v-else-if="authAny?.type === 'oauth2'" class="oauth-form">
        <p class="oauth-hint">
          <span class="oauth-status" :class="{ ok: oauthStatus !== '未授权' }">{{ oauthStatus }}</span>
          <button
            class="rf-btn rf-btn-sm"
            type="button"
            :disabled="authorizing"
            @click="oauthAuthorize"
          >
            {{ authorizing ? '授权中…' : '立即授权' }}
          </button>
        </p>
        <div class="kv-row">
          <input v-model="authAny.client_id" class="rf-input rf-input-sm kv-key" placeholder="Client ID" />
          <input v-model="authAny.client_secret" class="rf-input rf-input-sm kv-value" placeholder="Client Secret" type="password" />
        </div>
        <div class="kv-row">
          <input v-model="authAny.auth_url" class="rf-input rf-input-sm kv-key" placeholder="Authorize URL" />
          <input v-model="authAny.token_url" class="rf-input rf-input-sm kv-value" placeholder="Token URL" />
        </div>
        <div class="kv-row">
          <input v-model="authAny.scope" class="rf-input rf-input-sm kv-key" placeholder="Scope（空格分隔）" />
          <input v-model="authAny.redirect_uri" class="rf-input rf-input-sm kv-value" placeholder="Redirect URI" />
        </div>
      </div>
      <div v-else-if="authAny?.type === 'apikey'" class="kv-row">
        <input v-model="authAny.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
        <input
          v-model="authAny.value"
          class="rf-input rf-input-sm kv-value"
          placeholder="Value"
          spellcheck="false"
        />
        <select v-model="authAny.in" class="rf-input rf-input-sm auth-in-select">
          <option value="header">Header</option>
          <option value="query">Query</option>
        </select>
      </div>
    </div>

    <div class="editor-section">
      <h3 class="section-title">请求头 (Headers)</h3>
      <div v-for="(h, i) in draft.request.headers" :key="i" class="kv-row">
        <input v-model="h.enabled" type="checkbox" class="kv-check" />
        <input v-model="h.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
        <input v-model="h.value" class="rf-input rf-input-sm kv-value" placeholder="Value" />
        <button class="rf-btn rf-btn-sm kv-remove" type="button" @click="removeHeader(i)">✕</button>
      </div>
      <button class="rf-btn rf-btn-sm" type="button" @click="addHeader">＋ 添加请求头</button>
    </div>

    <div class="editor-section">
      <h3 class="section-title">请求体 (Body)</h3>
      <select
        :value="bodyAny?.mode ?? 'none'"
        class="rf-input rf-input-sm body-mode-select"
        @change="setBodyMode(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="m in BODY_MODES" :key="m.value" :value="m.value">{{ m.label }}</option>
      </select>
      <textarea
        v-if="bodyAny?.mode === 'json' || bodyAny?.mode === 'text'"
        v-model="bodyAny.raw"
        class="rf-input body-input"
        spellcheck="false"
        placeholder='{ "key": "value" }'
      ></textarea>
      <div v-else-if="bodyAny?.mode === 'graphql'" class="gql-editor">
        <textarea
          v-model="graphql.query"
          class="rf-input body-input"
          spellcheck="false"
          placeholder="query Hero($id: ID!) { hero(id: $id) { name } }"
        ></textarea>
        <textarea
          v-model="graphql.variables"
          class="rf-input body-input gql-vars"
          spellcheck="false"
          placeholder='{ "id": "42" }'
        ></textarea>
        <input
          v-model="graphql.operation_name"
          class="rf-input rf-input-sm"
          placeholder="operationName（可选）"
        />
      </div>
      <div v-else-if="bodyAny?.mode === 'urlencoded'" class="editor-fields">
        <div v-for="(f, i) in bodyAny.fields" :key="i" class="kv-row">
          <input v-model="f.enabled" type="checkbox" class="kv-check" />
          <input v-model="f.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
          <input v-model="f.value" class="rf-input rf-input-sm kv-value" placeholder="Value" />
          <button class="rf-btn rf-btn-sm kv-remove" type="button" @click="removeUrlencodedField(i)">✕</button>
        </div>
        <button class="rf-btn rf-btn-sm" type="button" @click="addUrlencodedField">＋ 添加字段</button>
      </div>
      <div v-else-if="bodyAny?.mode === 'multipart'" class="editor-fields">
        <div v-for="(f, i) in bodyAny.fields" :key="i" class="kv-row">
          <input v-model="f.enabled" type="checkbox" class="kv-check" />
          <input v-model="f.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
          <select v-model="f.value_type" class="rf-input rf-input-sm mp-type">
            <option value="text">文本</option>
            <option value="file_path">文件路径</option>
          </select>
          <input
            v-model="f.value"
            class="rf-input rf-input-sm kv-value"
            :placeholder="f.value_type === 'file_path' ? '/path/to/file' : 'Value'"
          />
          <button class="rf-btn rf-btn-sm kv-remove" type="button" @click="removeMultipartField(i)">✕</button>
        </div>
        <button class="rf-btn rf-btn-sm" type="button" @click="addMultipartField">＋ 添加字段</button>
      </div>
      <p v-else-if="bodyAny?.mode !== 'none'" class="body-hint">暂不支持该 Body 模式。</p>
    </div>

    <div v-if="sendError" class="send-error" role="alert">
      <span>发送失败：{{ sendError }}</span>
    </div>
    <div v-if="response" class="response" :class="{ error: response.status >= 400 }">
      <div class="response-head">
        <span class="response-status">{{ response.status }}</span>
        <span class="response-meta">{{ response.duration_ms }} ms · {{ response.size_bytes }} B</span>
        <span class="response-type">{{ response.content_type }}</span>
        <button class="rf-btn rf-btn-sm response-save" type="button" @click="saveExample">
          保存为示例
        </button>
      </div>
      <pre class="response-body">{{ prettyBody(response.body) }}</pre>
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
        <button class="tree-btn danger" type="button" title="删除" @click="removeExample(ex)">✕</button>
      </div>
      <pre v-if="viewingExample" class="example-body">{{ prettyBody(viewingExample.body) }}</pre>
    </div>

    <div v-if="histories.length" class="editor-section">
      <h3 class="section-title">请求历史 (最近 {{ histories.length }})</h3>
      <div v-for="h in histories" :key="h.id" class="history-row">
        <span class="history-method" :class="`m-select-${h.method.toLowerCase()}`">{{ h.method }}</span>
        <span class="history-url" :title="h.url">{{ h.url }}</span>
        <span v-if="historySummary(h)" class="history-status" :class="{ err: historySummary(h)!.status >= 400 }">
          {{ historySummary(h)!.status }}
        </span>
        <span class="history-meta">{{ h.duration_ms ?? '-' }} ms</span>
        <span class="history-meta">{{ h.created_at.slice(5, 16).replace('T', ' ') }}</span>
      </div>
    </div>

    <div class="editor-section">
      <h3 class="section-title">生成代码</h3>
      <div class="kv-row">
        <select v-model="codeLang" class="rf-input rf-input-sm code-lang-select">
          <option v-for="l in CODE_LANGS" :key="l.value" :value="l.value">{{ l.label }}</option>
        </select>
        <button class="rf-btn rf-btn-sm" type="button" :disabled="generating" @click="generateCode">
          {{ generating ? '生成中…' : '生成' }}
        </button>
        <button class="rf-btn rf-btn-sm" type="button" :disabled="!generatedCode" @click="copyCode">
          复制
        </button>
      </div>
      <pre v-if="generatedCode" class="code-preview">{{ generatedCode }}</pre>
    </div>

    <div class="editor-section">
      <h3 class="section-title">测试 (断言)</h3>
      <textarea
        v-model="testsJson"
        class="rf-input body-input"
        spellcheck="false"
        placeholder='{ "assertions": [{ "type": "status", "op": "eq", "expected": 200 }] }'
      ></textarea>
      <div class="kv-row">
        <button class="rf-btn rf-btn-sm" type="button" :disabled="testing" @click="runTests">
          {{ testing ? '测试中…' : '运行测试' }}
        </button>
        <span v-if="testResult" class="test-badge" :class="testResult.ok ? 'ok' : 'fail'">
          {{ testResult.ok ? '通过' : '失败' }} · {{ testResult.status ?? '-' }} · {{ testResult.duration_ms ?? '-' }} ms
        </span>
      </div>
      <ul v-if="testResult?.outcomes.length" class="outcome-list">
        <li
          v-for="(o, i) in testResult.outcomes"
          :key="i"
          class="outcome-row"
          :class="o.passed ? 'ok' : 'fail'"
        >
          <span>{{ o.passed ? '✓' : '✗' }}</span>
          <span class="outcome-text">{{ o.description }}</span>
          <span v-if="o.reason" class="outcome-reason">{{ o.reason }}</span>
        </li>
      </ul>
      <p v-else-if="testResult && !testResult.ok" class="test-hint">
        {{ testResult.request_error ?? '未配置断言' }}
      </p>
    </div>

    <div class="editor-section">
      <h3 class="section-title">压测 (并发基准)</h3>
      <div class="kv-row">
        <input v-model="loadConcurrency" class="rf-input rf-input-sm load-num" type="number" min="1" placeholder="并发" />
        <input v-model="loadTotal" class="rf-input rf-input-sm load-num" type="number" min="1" placeholder="总请求数" />
        <button class="rf-btn rf-btn-sm" type="button" :disabled="loading" @click="runLoadTest">
          {{ loading ? `压测中… ${loadProgress ? `${loadProgress.done}/${loadProgress.total}` : ''}` : '开始压测' }}
        </button>
      </div>
      <div v-if="loadProgress" class="load-progress">
        <div class="load-bar">
          <div
            class="load-bar-fill"
            :style="{ width: `${Math.round((loadProgress.done / loadProgress.total) * 100)}%` }"
          ></div>
        </div>
        <span class="load-progress-text">
          {{ loadProgress.done }}/{{ loadProgress.total }} · 成功 {{ loadProgress.ok }} · 失败 {{ loadProgress.failed }}
        </span>
      </div>
      <p v-if="loadResult" class="load-summary">
        {{ loadResult.total }} 次 · 成功 {{ loadResult.ok }} · 失败 {{ loadResult.failed }} ·
        耗时 {{ loadResult.total_ms }} ms · {{ loadResult.rps.toFixed(1) }} req/s ·
        p50 {{ loadResult.p50_ms }}ms · p90 {{ loadResult.p90_ms }}ms · p99 {{ loadResult.p99_ms }}ms
      </p>
    </div>

    <p v-if="!sendError && !generatedCode" class="editor-hint">
      填写请求后按 ⌘⏎（Ctrl+Enter）发送。响应正文过长时后端自动截断。
    </p>
  </div>
  <div v-else class="editor-empty">
    <p>从左侧选择接口开始编辑</p>
  </div>
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

.m-select-get { color: #34d399; }
.m-select-post { color: #fbbf24; }
.m-select-put { color: #93c5fd; }
.m-select-delete { color: #fca5a5; }
.m-select-patch { color: #c4b5fd; }

.path-input {
  flex: 1;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
}

.name-input {
  flex: 1;
}

.base-input {
  flex: 1.4;
  color: var(--rf-text-secondary, #9ca3af);
}

.editor-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}

.editor-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.section-title {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--rf-text-secondary, #9ca3af);
}

.kv-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.kv-check {
  accent-color: #3b82f6;
}

.kv-key {
  width: 220px;
}

.kv-value {
  flex: 1;
}

.kv-remove {
  color: var(--rf-text-muted, #6b7280);
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
  color: var(--rf-text-muted, #6b7280);
}

.oauth-status.ok {
  color: #34d399;
}

.code-lang-select {
  width: 180px;
}

.history-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  padding: 3px 0;
  border-bottom: 1px dashed var(--rf-border, #1f2937);
}

.history-method {
  width: 52px;
  flex-shrink: 0;
  font-weight: 700;
}

.history-url {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--rf-text-secondary, #9ca3af);
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
}

.history-status {
  color: #34d399;
  font-weight: 600;
}

.history-status.err {
  color: #f87171;
}

.history-meta {
  color: var(--rf-text-muted, #6b7280);
  flex-shrink: 0;
}

.test-badge {
  font-size: 12px;
  font-weight: 600;
}

.test-badge.ok {
  color: #34d399;
}

.test-badge.fail {
  color: #f87171;
}

.test-hint {
  margin: 0;
  font-size: 12px;
  color: var(--rf-text-muted, #6b7280);
}

.outcome-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 12px;
}

.outcome-row {
  display: flex;
  gap: 8px;
  align-items: baseline;
}

.outcome-row.ok {
  color: #34d399;
}

.outcome-row.fail {
  color: #f87171;
}

.outcome-text {
  flex: 1;
  word-break: break-all;
}

.outcome-reason {
  color: var(--rf-text-muted, #6b7280);
  word-break: break-all;
}

.load-num {
  width: 110px;
}

.load-summary {
  margin: 0;
  font-size: 12px;
  color: var(--rf-text-secondary, #9ca3af);
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
}

.load-progress {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.load-bar {
  height: 6px;
  border-radius: 3px;
  background: var(--rf-input-bg, #0f172a);
  overflow: hidden;
}

.load-bar-fill {
  height: 100%;
  background: #3b82f6;
  transition: width 0.15s ease;
}

.load-progress-text {
  font-size: 11.5px;
  color: var(--rf-text-muted, #6b7280);
}

.code-preview {
  margin: 0;
  padding: 10px 12px;
  border: 1px solid var(--rf-border, #1f2937);
  border-radius: 6px;
  background: var(--rf-input-bg, #0f172a);
  color: var(--rf-text, #f9fafb);
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12px;
  line-height: 1.55;
  overflow: auto;
  max-height: 320px;
  white-space: pre-wrap;
  word-break: break-all;
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
  border: 1px solid var(--rf-border, #1f2937);
  background: var(--rf-input-bg, #0f172a);
  border-radius: 6px;
  padding: 5px 10px;
  cursor: pointer;
  color: var(--rf-text, #f9fafb);
  font-size: 12.5px;
  text-align: left;
}

.example-main.open {
  border-color: #3b82f6;
}

.example-status {
  font-weight: 700;
  font-size: 11px;
  color: #34d399;
}

.example-status.err {
  color: #fca5a5;
}

.example-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.example-meta {
  font-size: 11px;
  color: var(--rf-text-muted, #6b7280);
}

.example-body {
  margin: 0;
  padding: 10px 12px;
  background: var(--rf-input-bg, #0f172a);
  border: 1px solid var(--rf-border, #1f2937);
  border-radius: 6px;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 240px;
  overflow-y: auto;
}

.env-select {
  width: 180px;
  flex-shrink: 0;
}

.gql-vars {
  min-height: 80px;
}

.body-hint {
  margin: 0;
  font-size: 12px;
  color: var(--rf-text-muted, #6b7280);
}

.auth-type-select {
  width: 160px;
}

.auth-in-select {
  width: 100px;
}

.body-input {
  width: 100%;
  min-height: 120px;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12.5px;
  resize: vertical;
}

.send-error {
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.35);
  color: #fca5a5;
  font-size: 12.5px;
}

.response {
  border: 1px solid var(--rf-border, #1f2937);
  border-radius: 8px;
  background: var(--rf-input-bg, #0f172a);
  overflow: hidden;
}

.response.error {
  border-color: rgba(239, 68, 68, 0.45);
}

.response-head {
  display: flex;
  gap: 12px;
  align-items: center;
  padding: 8px 12px;
  border-bottom: 1px solid var(--rf-border, #1f2937);
}

.response-status {
  font-weight: 700;
  font-size: 13px;
  color: #34d399;
}

.response.error .response-status {
  color: #fca5a5;
}

.response-meta,
.response-type {
  font-size: 11.5px;
  color: var(--rf-text-muted, #6b7280);
}

.response-body {
  margin: 0;
  padding: 12px;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12.5px;
  line-height: 1.5;
  color: var(--rf-text, #f9fafb);
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 320px;
  overflow-y: auto;
}

.editor-hint {
  margin: 0;
  font-size: 12px;
  color: var(--rf-text-muted, #6b7280);
}

.editor-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--rf-text-muted, #6b7280);
}
</style>
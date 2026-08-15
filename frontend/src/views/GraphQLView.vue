<script setup lang="ts">
/**
 * GraphQLView：GraphQL 接口编辑与调试视图。
 *
 * - 左：Query 编辑器（语法高亮，覆盖层方案：透明 textarea + 高亮 pre）
 * - 右：Variables JSON 编辑器 + operationName
 * - 顶栏：保存 / 发送 / 历史 / 生成代码
 * - 发送走后端 execute_request（POST + application/json），
 *   响应区分 data 与 errors（GraphQL 语义，errors 优先渲染）
 * - 生成代码在浏览器端完成，算法与 fox-codegen 的 render_graphql_curl /
 *   render_graphql_js 保持一致
 *
 * 样式沿用项目 rf- 设计系统（深色 slate 主题，变量名与
 * crates/fox-desktop/src/styles.rs 的 DESIGN_SYSTEM_CSS 对齐）。
 */
import { computed, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import { useWorkspaceStore } from '../stores/workspace'
import type { BodySpec, ExecuteRequestArgs, ExecuteResponse, GraphQLSpec } from '../types/foxApi'

const router = useRouter()

const props = withDefaults(
  defineProps<{
    url?: string
    environmentId?: string | null
    /** 当前接口的 BodySpec（mode !== 'graphql' 时使用默认空模板） */
    body?: BodySpec
  }>(),
  { url: '', environmentId: null, body: undefined },
)

const emit = defineEmits<{
  (e: 'save', body: BodySpec): void
}>()

const store = useWorkspaceStore()

const api = useFoxApi()
const toast = useToast()

// ---------- 编辑器状态 ----------
function initSpec(): GraphQLSpec {
  if (props.body?.mode === 'graphql') {
    return { ...props.body.spec }
  }
  return { query: '', variables: '{}', operation_name: '' }
}

const gql = ref<GraphQLSpec>(initSpec())
const url = ref(props.url ?? '')
const sending = ref(false)
const queryEditor = ref<HTMLElement | null>(null)
const varsEditor = ref<HTMLElement | null>(null)
const saving = ref(false)
const statusText = ref('')
const response = ref<{ status: number; body: string; durationMs: number } | null>(null)
const responseErrors = ref<{ message: string; locations?: string; path?: string }[]>([])
const responseRaw = ref(false)
const sendFailed = ref(false)

// ---------- 变量 JSON 校验 ----------
function parseVariables(text: string): unknown | null {
  const trimmed = text.trim()
  if (!trimmed || trimmed === '{}') return {}
  try {
    const value = JSON.parse(trimmed)
    if (value !== null && typeof value === 'object' && !Array.isArray(value)) return value
    return null
  } catch {
    return null
  }
}

const variablesValid = computed(() => {
  const t = gql.value.variables.trim()
  if (!t || t === '{}') return true
  try {
    const v = JSON.parse(t)
    return v !== null && typeof v === 'object' && !Array.isArray(v)
  } catch {
    return false
  }
})

// ---------- 高亮 ----------
const KEYWORDS =
  'query|mutation|subscription|fragment|on|schema|scalar|type|interface|union|enum|input|implements|directive|extend|true|false|null'

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

/** 轻量 GraphQL 高亮（注释 / 字符串 / 变量 / 关键字 / 数字）。 */
function highlightGraphQL(code: string): string {
  const re =
    /(#[^\n]*)|("(?:[^"\\]|\\.)*"|"""(?:.|\n)*?""")|(\$[A-Za-z_][A-Za-z0-9_]*)|([A-Za-z_][A-Za-z0-9_]*)|(-?\b\d+(?:\.\d+)?\b)|([{}\[\]():!=\|&,.])/g
  let out = ''
  let last = 0
  for (const m of code.matchAll(re)) {
    out += escapeHtml(code.slice(last, m.index))
    const [full, comment, str, variable, ident, num, _punct] = m
    if (comment) out += `<span class="hl-c">${escapeHtml(full)}</span>`
    else if (str) out += `<span class="hl-s">${escapeHtml(full)}</span>`
    else if (variable) out += `<span class="hl-v">${escapeHtml(full)}</span>`
    else if (ident) {
      if (KEYWORDS.split('|').includes(ident)) {
        out += `<span class="hl-k">${escapeHtml(full)}</span>`
      } else {
        out += escapeHtml(full)
      }
    } else if (num) out += `<span class="hl-n">${escapeHtml(full)}</span>`
    else out += `<span class="hl-p">${escapeHtml(full)}</span>`
    last = m.index! + full.length
  }
  out += escapeHtml(code.slice(last))
  return out
}

/** 轻量 JSON 高亮（键 / 字符串 / 数字 / 布尔 / null）。 */
function highlightJSON(code: string): string {
  const re =
    /("(?:[^"\\]|\\.)*")(\s*:)?|(-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b)|(true|false|null)/g
  let out = ''
  let last = 0
  for (const m of code.matchAll(re)) {
    out += escapeHtml(code.slice(last, m.index))
    const [full, str, colon, num, bool] = m
    if (str) out += `<span class="hl-s${colon ? ' hl-k' : ''}">${escapeHtml(full)}</span>`
    else if (num) out += `<span class="hl-n">${escapeHtml(full)}</span>`
    else if (bool) out += `<span class="hl-b">${escapeHtml(full)}</span>`
    last = m.index! + full.length
  }
  out += escapeHtml(code.slice(last))
  return out
}

const queryHtml = computed(() => highlightGraphQL(gql.value.query))
const variablesHtml = computed(() => {
  const t = gql.value.variables
  if (variablesValid.value) return highlightJSON(t)
  return escapeHtml(t)
})

const responseHtml = computed(() => {
  if (!response.value) return ''
  const text = response.value.body
  try {
    const parsed = JSON.parse(text)
    return highlightJSON(JSON.stringify(parsed, null, 2))
  } catch {
    return escapeHtml(text)
  }
})

const statusClass = computed(() => {
  if (!response.value) return ''
  if (response.value.status >= 400 || responseErrors.value.length) return 'status-err'
  return 'status-ok'
})

// ---------- 编辑器滚动同步 ----------
function syncScroll(e: Event, pre: HTMLElement | null) {
  const ta = e.target as HTMLElement
  if (!pre) return
  pre.scrollTop = ta.scrollTop
  pre.scrollLeft = ta.scrollLeft
}

// ---------- 构建请求 ----------
function buildArgs(): ExecuteRequestArgs {
  return {
    url: url.value.trim(),
    method: 'POST',
    spec: {
      params: [],
      headers: [],
      path_variables: [],
      auth: { type: 'none' },
      body: { mode: 'graphql', spec: { ...gql.value } },
      timeout_ms: 30000,
      follow_redirects: true,
      tests: null,
    },
    environment_id: props.environmentId ?? store.activeEnvId ?? null,
  }
}

// ---------- 发送 ----------
async function send() {
  if (!url.value.trim()) {
    statusText.value = '请输入接口地址'
    toast.warning('请输入接口地址')
    return
  }
  if (!gql.value.query.trim()) {
    statusText.value = '请输入 GraphQL Query'
    toast.warning('请输入 GraphQL Query')
    return
  }
  if (!variablesValid.value) {
    statusText.value = 'Variables 不是合法 JSON 对象'
    toast.warning('Variables 不是合法 JSON 对象')
    return
  }
  statusText.value = ''
  sending.value = true
  sendFailed.value = false
  responseErrors.value = []
  try {
    const res: ExecuteResponse = await api.executeRequest(buildArgs())
    response.value = {
      status: res.status,
      body: res.body,
      durationMs: res.duration_ms,
    }
    // GraphQL 语义：errors 字段存在即业务失败，优先展示
    responseErrors.value = parseResponseErrors(res.body)
    pushHistory()
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e)
    statusText.value = message
    responseRaw.value = true
    sendFailed.value = true
    toast.error('请求发送失败', { message, duration: 6000 })
  } finally {
    sending.value = false
  }
}

/** 发送失败后的重试：上次请求参数原样重发。 */
async function retrySend() {
  await send()
}

function parseResponseErrors(body: string): { message: string; locations?: string; path?: string }[] {
  try {
    const parsed = JSON.parse(body)
    if (parsed && typeof parsed === 'object' && Array.isArray(parsed.errors)) {
      return parsed.errors.map((entry: Record<string, unknown>) => ({
        message: String(entry.message ?? '未知错误'),
        locations: Array.isArray(entry.locations)
          ? entry.locations
              .map((loc: Record<string, unknown>) => `${loc.line}:${loc.column}`)
              .join(', ')
          : undefined,
        path: Array.isArray(entry.path) ? entry.path.join('.') : undefined,
      }))
    }
  } catch {
    // 非 JSON 响应交给原始响应区展示
  }
  return []
}

// ---------- 保存 ----------
function save() {
  if (!variablesValid.value) {
    statusText.value = 'Variables 不是合法 JSON 对象'
    return
  }
  saving.value = true
  try {
    emit('save', { mode: 'graphql', spec: { ...gql.value } })
    pushHistory()
    statusText.value = '已保存'
    toast.success('GraphQL 请求已保存')
  } finally {
    saving.value = false
  }
}

// ---------- 历史（localStorage） ----------
interface HistoryEntry {
  query: string
  variables: string
  operation_name: string
  when: string
}

const HISTORY_KEY = 'rustfox_graphql_history'
const MAX_HISTORY = 20

function loadHistory(): HistoryEntry[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY)
    return raw ? (JSON.parse(raw) as HistoryEntry[]) : []
  } catch {
    return []
  }
}

function pushHistory() {
  const entries = loadHistory()
  const entry: HistoryEntry = {
    query: gql.value.query,
    variables: gql.value.variables,
    operation_name: gql.value.operation_name,
    when: new Date().toISOString(),
  }
  entries.unshift(entry)
  const deduped = entries.filter(
    (e, i) => i === 0 || e.query !== entries[i - 1].query || e.variables !== entries[i - 1].variables,
  )
  localStorage.setItem(HISTORY_KEY, JSON.stringify(deduped.slice(0, MAX_HISTORY)))
}

const historyOpen = ref(false)
const history = ref<HistoryEntry[]>([])

function openHistory() {
  history.value = loadHistory()
  historyOpen.value = true
}

function applyHistory(entry: HistoryEntry) {
  gql.value = { ...entry }
  historyOpen.value = false
}

function clearHistory() {
  localStorage.removeItem(HISTORY_KEY)
  history.value = []
}

// ---------- 生成代码（与 fox-codegen 保持一致） ----------
const codegenOpen = ref(false)
const codegenTab = ref<'curl' | 'apollo'>('curl')
const copied = ref(false)

function sq(s: string): string {
  return s.replace(/'/g, "'\\''")
}

function dq(s: string): string {
  return s.replace(/\\/g, '\\\\').replace(/"/g, '\\"').replace(/\n/g, '\\n').replace(/\r/g, '\\r').replace(/\t/g, '\\t')
}

function gqlJsonBody(): string {
  const payload: Record<string, unknown> = { query: gql.value.query }
  const vars = parseVariables(gql.value.variables)
  if (vars && Object.keys(vars as object).length > 0) payload.variables = vars
  if (gql.value.operation_name.trim()) payload.operationName = gql.value.operation_name.trim()
  return JSON.stringify(payload)
}

const curlCode = computed(() => {
  let out = `curl -X POST '${sq(url.value.trim())}'`
  out += ` \\\n     -H 'Content-Type: application/json'`
  out += ` \\\n     --data '${sq(gqlJsonBody())}'`
  return out
})

const apolloCode = computed(() => {
  let out = `import { ApolloClient, InMemoryCache, gql } from '@apollo/client';\n\n`
  out += `const client = new ApolloClient({\n`
  out += `  uri: '${sq(url.value.trim())}',\n`
  out += `  cache: new InMemoryCache(),\n`
  out += `});\n\n`
  out += `const QUERY = gql\`\n${gql.value.query}\n\`;\n\n`
  const vars = parseVariables(gql.value.variables) ?? {}
  out += `const variables = ${JSON.stringify(vars)};\n\n`
  const op = gql.value.operation_name.trim()
  if (op) {
    out += `const result = await client.query({\n  query: QUERY,\n  variables,\n  operationName: '${dq(op)}',\n});\n`
  } else {
    out += `const result = await client.query({\n  query: QUERY,\n  variables,\n});\n`
  }
  out += `console.log(result.data);\n`
  return out
})

function openCodegen() {
  codegenOpen.value = true
  copied.value = false
  codegenTab.value = 'curl'
}

async function copyCode() {
  const text = codegenTab.value === 'curl' ? curlCode.value : apolloCode.value
  try {
    await navigator.clipboard.writeText(text)
    copied.value = true
    setTimeout(() => (copied.value = false), 1500)
  } catch {
    statusText.value = '复制失败，请手动选择'
  }
}
</script>

<template>
  <div class="gql-root">
    <div class="row rf-mb-2">
      <button class="rf-btn rf-btn-sm" type="button" @click="router.push('/workspace')">← 返回工作区</button>
      <input v-model="url" class="rf-input gql-url" placeholder="GraphQL 接口地址（支持 &#123;&#123;变量&#125;&#125; 与环境替换）" spellcheck="false" />
      <button class="rf-btn rf-btn-sm" :disabled="saving" @click="save">保存</button>
      <button class="rf-btn rf-btn-sm rf-btn-primary" :disabled="sending" @click="send">
        {{ sending ? '发送中…' : '发送' }}
      </button>
      <button
        v-if="sendFailed"
        class="rf-btn rf-btn-sm rf-btn-ghost"
        :disabled="sending"
        @click="retrySend"
      >
        {{ sending ? '重试中…' : '重试' }}
      </button>
      <button class="rf-btn rf-btn-sm rf-btn-ghost" @click="openHistory">历史</button>
      <button class="rf-btn rf-btn-sm rf-btn-ghost" @click="openCodegen">生成代码</button>
    </div>

    <div class="gql-grid">
      <div class="gql-pane">
        <div class="pane-title">
          <span>Query</span>
          <span class="hint-inline">GraphQL 查询（支持 &#123;&#123;变量&#125;&#125; 插值）</span>
        </div>
        <div class="hl-wrap">
          <pre class="hl-pre" aria-hidden="true" v-html="queryHtml"></pre>
          <textarea
            ref="queryEditor"
            class="hl-ta"
            :value="gql.query"
            spellcheck="false"
            placeholder="query Hero($id: ID!) {&#10;  hero(id: $id) { name }&#10;}"
            @input="gql.query = ($event.target as HTMLTextAreaElement).value"
            @scroll="syncScroll($event, (queryEditor as HTMLElement | null))"
          ></textarea>
        </div>
      </div>

      <div class="gql-pane">
        <div class="pane-title">
          <span>Variables</span>
          <span class="hint-inline" :class="{ 'vars-invalid': !variablesValid }">
            {{ variablesValid ? 'JSON 对象' : 'JSON 无效' }}
          </span>
        </div>
        <div class="hl-wrap">
          <pre class="hl-pre" aria-hidden="true" v-html="variablesHtml"></pre>
          <textarea
            ref="varsEditor"
            class="hl-ta"
            :value="gql.variables"
            spellcheck="false"
            placeholder='{"id": "42"}'
            @input="gql.variables = ($event.target as HTMLTextAreaElement).value"
            @scroll="syncScroll($event, (varsEditor as HTMLElement | null))"
          ></textarea>
        </div>
        <label class="op-name-label">
          operationName
          <input
            v-model="gql.operation_name"
            class="rf-input rf-input-sm"
            placeholder="可选，多操作时指定"
            spellcheck="false"
          />
        </label>
      </div>
    </div>

    <div class="resp-head">
      <div class="hint-inline" :class="statusClass">
        {{ statusText || (response ? `${response.status} · ${response.durationMs}ms` : '') }}
      </div>
      <button v-if="response" class="rf-btn rf-btn-sm rf-btn-ghost" @click="responseRaw = !responseRaw">
        {{ responseRaw ? '格式化' : '原始' }}
      </button>
    </div>

    <div v-if="responseErrors.length" class="resp-errors">
      <div v-for="(err, i) in responseErrors" :key="i" class="resp-error">
        <span class="err-msg">{{ err.message }}</span>
        <span v-if="err.locations" class="hint-inline">位置 {{ err.locations }}</span>
        <span v-if="err.path" class="hint-inline">路径 {{ err.path }}</span>
      </div>
    </div>

    <div v-if="response" class="resp-body">
      <pre v-if="!responseRaw" class="resp-pre" v-html="responseHtml"></pre>
      <pre v-else class="resp-pre">{{ response.body }}</pre>
    </div>
    <div v-else class="resp-empty">
      <p class="hint">发送请求后在此查看响应（data / errors 区分展示）</p>
    </div>

    <!-- 历史 -->
    <div v-if="historyOpen" class="modal-backdrop" @click.self="historyOpen = false">
      <div class="modal">
        <div class="modal-head">
          <span class="modal-title">历史记录</span>
          <button class="rf-btn rf-btn-sm rf-btn-ghost" @click="historyOpen = false">关闭</button>
        </div>
        <div class="modal-body">
          <div v-if="!history.length" class="empty">暂无历史</div>
          <ul v-else class="history-list">
            <li v-for="(entry, i) in history" :key="i" class="history-item" @click="applyHistory(entry)">
              <pre class="history-query">{{ entry.query }}</pre>
              <span class="hint-inline">{{ new Date(entry.when).toLocaleString() }}</span>
            </li>
          </ul>
        </div>
        <div class="modal-foot">
          <button class="rf-btn rf-btn-sm rf-btn-danger" @click="clearHistory">清空历史</button>
        </div>
      </div>
    </div>

    <!-- 生成代码 -->
    <div v-if="codegenOpen" class="modal-backdrop" @click.self="codegenOpen = false">
      <div class="modal">
        <div class="modal-head">
          <span class="modal-title">生成代码</span>
          <button class="rf-btn rf-btn-sm rf-btn-ghost" @click="codegenOpen = false">关闭</button>
        </div>
        <div class="modal-body">
          <div class="rf-tabs">
            <button class="rf-tab" :class="{ active: codegenTab === 'curl' }" @click="codegenTab = 'curl'">curl</button>
            <button class="rf-tab" :class="{ active: codegenTab === 'apollo' }" @click="codegenTab = 'apollo'">JavaScript (Apollo Client)</button>
          </div>
          <pre class="codegen-out">{{ codegenTab === 'curl' ? curlCode : apolloCode }}</pre>
        </div>
        <div class="modal-foot">
          <button class="rf-btn rf-btn-sm rf-btn-primary" @click="copyCode">{{ copied ? '已复制' : '复制' }}</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 私有色板映射到全局 rf-* 令牌，随主题联动 */
.gql-root {
  --bg: var(--rf-bg);
  --panel: var(--rf-bg-panel);
  --panel-2: var(--rf-bg-panel-2);
  --border: var(--rf-border);
  --border-2: var(--rf-text-muted);
  --text: var(--rf-text);
  --text-2: var(--rf-text-secondary);
  --muted: var(--rf-text-muted);
  --accent: var(--rf-info);
  --accent-2: var(--rf-accent-weak);
  --accent-soft: var(--rf-info-tint);
  --accent-line: rgba(59, 130, 246, 0.45);
  --success: var(--rf-success);
  --warning: var(--rf-warning);
  --danger: var(--rf-danger);
  --danger-weak: var(--rf-danger-tint);
  --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, 'Liberation Mono', monospace;
  --r-s: var(--rf-radius-sm);
  --s-2: 8px;
  --s-3: 12px;
  --s-4: 16px;
  color: var(--text);
  background: var(--bg);
  font-size: 13.5px;
}

.row {
  display: flex;
  align-items: center;
  gap: var(--s-2);
}

.rf-mb-2 {
  margin-bottom: var(--s-2);
}

.rf-input-sm {
  flex: 1;
  min-width: 0;
  padding: 4px 8px;
}

.rf-btn-ghost {
  background: transparent;
  border-color: var(--border);
  color: var(--text-2);
}

.rf-btn-ghost:hover:not(:disabled) {
  background: var(--panel-2);
  color: var(--text);
  border-color: var(--border-2);
}

.rf-btn-danger {
  background: var(--danger);
  color: #fff;
  font-weight: 600;
  border-color: transparent;
}

.rf-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: var(--s-3);
  border-bottom: 1px solid var(--border);
}

.rf-tab {
  padding: 6px 12px;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-2);
  cursor: pointer;
  font-size: 13px;
}

.rf-tab:hover {
  color: var(--text);
}

.rf-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(2, 6, 17, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.modal {
  width: min(720px, 90vw);
  max-height: 82vh;
  display: flex;
  flex-direction: column;
  background: var(--panel);
  border: 1px solid var(--border-2);
  border-radius: 10px;
  box-shadow: 0 16px 48px rgba(2, 6, 17, 0.5);
}

.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--s-3) var(--s-4);
  border-bottom: 1px solid var(--border);
}

.modal-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--accent);
}

.modal-body {
  padding: var(--s-4);
  overflow: auto;
  flex: 1;
  min-height: 0;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  gap: var(--s-2);
  padding: var(--s-3) var(--s-4);
  border-top: 1px solid var(--border);
}

/* ---------- 布局 ---------- */
.gql-url {
  flex: 1;
  min-width: 0;
}

.gql-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--s-3);
}

.gql-pane {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--s-2);
}

.pane-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-2);
  font-weight: 600;
}

.hint-inline {
  color: var(--muted);
  font-size: 12px;
}

.vars-invalid {
  color: var(--danger);
}

/* ---------- 高亮编辑器（透明 textarea + pre 覆盖层） ---------- */
.hl-wrap {
  position: relative;
  background: var(--rf-input-bg);
  border: 1px solid var(--border);
  border-radius: var(--r-s);
  height: 300px;
  overflow: hidden;
}

.hl-wrap:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}

.hl-pre,
.hl-ta {
  margin: 0;
  padding: 10px 12px;
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.55;
  white-space: pre;
  tab-size: 2;
  overflow: hidden;
}

.hl-pre {
  position: absolute;
  inset: 0;
  color: var(--text);
  pointer-events: none;
  overflow: auto;
}

.hl-ta {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  color: transparent;
  caret-color: var(--accent);
  overflow: auto;
}

.hl-ta::selection {
  background: var(--accent-soft);
}

.hl-c {
  color: var(--muted);
  font-style: italic;
}

.hl-s {
  color: var(--success);
}

.hl-k {
  color: var(--accent);
  font-weight: 600;
}

.hl-v {
  color: var(--warning);
}

.hl-n {
  color: #c084fc;
}

.hl-p {
  color: var(--text-2);
}

.hl-b {
  color: var(--danger);
}

.op-name-label {
  display: flex;
  align-items: center;
  gap: var(--s-2);
  font-size: 12px;
  color: var(--text-2);
}

/* ---------- 响应区 ---------- */
.resp-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: var(--s-4);
}

.status-ok {
  color: var(--success);
}

.status-err {
  color: var(--danger);
  font-weight: 700;
}

.resp-errors {
  margin-top: var(--s-2);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.resp-error {
  padding: 8px 12px;
  border: 1px solid var(--danger-weak);
  background: var(--danger-weak);
  border-radius: var(--r-s);
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.err-msg {
  color: var(--danger);
  font-weight: 600;
  font-family: var(--mono);
  font-size: 12.5px;
}

.resp-body {
  margin-top: var(--s-2);
  background: var(--rf-input-bg);
  border: 1px solid var(--border);
  border-radius: var(--r-s);
  max-height: 320px;
  overflow: auto;
}

.resp-pre {
  margin: 0;
  padding: 10px 12px;
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
}

.resp-empty {
  margin-top: var(--s-4);
  text-align: center;
}

.hint {
  color: var(--muted);
  font-size: 12px;
  text-align: center;
  margin-top: var(--s-2);
}

.empty {
  text-align: center;
  color: var(--muted);
  padding: var(--s-4) 0;
  font-size: 13px;
}

/* ---------- 历史 ---------- */
.history-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.history-item {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--r-s);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--s-3);
}

.history-item:hover {
  border-color: var(--accent-line);
  background: var(--accent-soft);
}

.history-query {
  margin: 0;
  flex: 1;
  min-width: 0;
  font-family: var(--mono);
  font-size: 12px;
  color: var(--text-2);
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 64px;
  overflow: auto;
}

/* ---------- 生成代码 ---------- */
.codegen-out {
  margin: 0;
  padding: 12px;
  background: var(--rf-input-bg);
  border: 1px solid var(--border);
  border-radius: var(--r-s);
  font-family: var(--mono);
  font-size: 12.5px;
  line-height: 1.55;
  white-space: pre;
  overflow: auto;
  max-height: 400px;
}
</style>

<script setup lang="ts">
/**
 * ResponsePanel：响应面板。
 * - 顶栏：高对比状态栏（2xx 实心绿 / 3xx 琥珀 / 4xx-5xx 红，`201 Created`）
 *   + 耗时 / 大小指标（竖线分隔）+ 类型；
 * - 工具栏：Body/Headers/Cookies 标签 + 格式化/原始/预览 分段切换（右）
 *   + 查找（⌘F）/ 展开-收起全部 / 保存为示例 / 复制响应（最右）；
 * - 查找：顶部弹出搜索框，高亮匹配 + 上一个/下一个导航（Enter / Shift+Enter / Esc）；
 * - 主体：JSON → 可折叠树形查看器（行号 + VS Code 深色语法着色）；文本 → 行号代码视图；HTML → 沙箱预览。
 */
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useToast } from '../composables/useToast'
import { escapeHtml, highlightJSONText } from '../utils/highlight'
import { EDITOR_INDENT } from '../constants/editorTheme'
import { formatBytes, formatDuration } from '../utils/format'
import FindBar from './ui/FindBar.vue'
import Icon from './ui/Icon.vue'
import JsonTree from './JsonTree.vue'
import SegmentedControl, { type SegmentOption } from './ui/SegmentedControl.vue'
import Tabs, { type TabItem } from './ui/Tabs.vue'
import Tooltip from './ui/Tooltip.vue'
import type { ExecuteResponse } from '../types/foxApi'

const props = defineProps<{ response: ExecuteResponse }>()

const emit = defineEmits<{ saveExample: [] }>()

const toast = useToast()

// ---------- 状态 ----------
const activeTab = ref<'body' | 'headers' | 'cookies'>('body')
type ViewMode = 'pretty' | 'raw' | 'preview'
const viewMode = ref<ViewMode>('pretty')

const REASON_PHRASES: Record<number, string> = {
  100: 'Continue',
  101: 'Switching Protocols',
  200: 'OK',
  201: 'Created',
  202: 'Accepted',
  204: 'No Content',
  301: 'Moved Permanently',
  302: 'Found',
  304: 'Not Modified',
  400: 'Bad Request',
  401: 'Unauthorized',
  403: 'Forbidden',
  404: 'Not Found',
  405: 'Method Not Allowed',
  409: 'Conflict',
  410: 'Gone',
  422: 'Unprocessable Entity',
  429: 'Too Many Requests',
  500: 'Internal Server Error',
  502: 'Bad Gateway',
  503: 'Service Unavailable',
  504: 'Gateway Timeout',
}

const tone = computed(() => {
  const s = props.response.status
  if (s < 300) return 'ok'
  if (s < 400) return 'warn'
  return 'err'
})

const statusText = computed(() => {
  const s = props.response.status
  return `${s} ${REASON_PHRASES[s] ?? (s < 400 ? 'OK' : 'Error')}`
})

const sizeText = computed(() => formatBytes(props.response.size_bytes))

const headerRows = computed(() => props.response.headers.map(([k, v]) => ({ k, v })))

// ---------- 正文解析 ----------
// 大响应保护：超过阈值跳过 JSON 解析与树形渲染（全量 parse/渲染会冻结 UI），
// 回退为按行文本视图；行渲染按块渐进加载（每次追加 LINE_CHUNK 行）。
const PARSE_LIMIT_BYTES = 1_000_000
const LINE_CHUNK = 1000
const visibleLines = ref(LINE_CHUNK)
watch(
  () => props.response,
  () => {
    visibleLines.value = LINE_CHUNK
    treeExpanded.value = false
  },
)

const parsed = computed<unknown | null>(() => {
  if (!props.response.body.trim()) return null
  if (props.response.body.length > PARSE_LIMIT_BYTES) return null
  try {
    return JSON.parse(props.response.body)
  } catch {
    return null
  }
})

const isJson = computed(() => parsed.value !== null)

const pretty = computed(() => {
  if (parsed.value !== null) return JSON.stringify(parsed.value, null, EDITOR_INDENT)
  return props.response.body
})

const isHtml = computed(() => props.response.content_type.toLowerCase().includes('html'))

/** 内容类型支持「预览」：HTML / 图片 / 音视频 / 二进制文件类。 */
const isPreviewable = computed(() =>
  /^(text\/html|image\/|audio\/|video\/|application\/(pdf|octet-stream|zip|x-zip|x-.*?zip|json))/i.test(
    props.response.content_type,
  ),
)
const isImage = computed(() => props.response.content_type.toLowerCase().startsWith('image/'))

const prettyLines = computed(() => pretty.value.split('\n'))
const rawLines = computed(() => props.response.body.split('\n'))
const shownPrettyLines = computed(() => prettyLines.value.slice(0, visibleLines.value))
const shownRawLines = computed(() => rawLines.value.slice(0, visibleLines.value))
const hasMorePretty = computed(() => prettyLines.value.length > visibleLines.value)
const hasMoreRaw = computed(() => rawLines.value.length > visibleLines.value)
const bodyTooLarge = computed(() => props.response.body.length > PARSE_LIMIT_BYTES)
function showMoreLines(): void {
  visibleLines.value += LINE_CHUNK
}

// ---------- Cookies（由 set-cookie 响应头解析） ----------
interface Cookie {
  name: string
  value: string
  domain: string
  path: string
  expires: string
  httpOnly: boolean
  secure: boolean
  sameSite: string
}

const cookies = computed<Cookie[]>(() => {
  const out: Cookie[] = []
  for (const [k, v] of props.response.headers) {
    if (k.toLowerCase() !== 'set-cookie') continue
    const parts = v.split(';').map((s) => s.trim())
    const [nv, ...attrs] = parts
    const eq = nv.indexOf('=')
    const cookie: Cookie = {
      name: eq > 0 ? nv.slice(0, eq).trim() : nv,
      value: eq > 0 ? nv.slice(eq + 1).trim() : '',
      domain: '',
      path: '',
      expires: '',
      httpOnly: false,
      secure: false,
      sameSite: '',
    }
    for (const a of attrs) {
      const i = a.indexOf('=')
      const key = (i > 0 ? a.slice(0, i) : a).toLowerCase()
      const val = i > 0 ? a.slice(i + 1).trim() : 'true'
      if (key === 'domain') cookie.domain = val
      else if (key === 'path') cookie.path = val
      else if (key === 'expires') cookie.expires = val
      else if (key === 'httponly') cookie.httpOnly = true
      else if (key === 'secure') cookie.secure = true
      else if (key === 'samesite') cookie.sameSite = val
    }
    out.push(cookie)
  }
  return out
})

const responseTabs = computed<TabItem[]>(() => [
  { key: 'body', label: 'Body' },
  { key: 'headers', label: 'Headers', count: headerRows.value.length },
  { key: 'cookies', label: 'Cookies', count: cookies.value.length },
])

// ---------- 操作 ----------
const copySource = computed(() =>
  viewMode.value === 'raw' || viewMode.value === 'preview'
    ? props.response.body
    : pretty.value,
)

async function copyBody(): Promise<void> {
  try {
    await navigator.clipboard.writeText(copySource.value)
    toast.success('已复制响应正文')
  } catch {
    toast.error('复制失败，请手动选择文本')
  }
}

const MODE_OPTIONS = computed<SegmentOption[]>(() => [
  { value: 'pretty', label: '格式化', icon: 'list' },
  { value: 'raw', label: '原始', icon: 'code' },
  ...(isPreviewable.value ? [{ value: 'preview', label: '预览', icon: 'eye' as const }] : []),
])

/** 响应类型不支持预览时，强制退回「格式化」，避免残留 preview 状态。 */
watch(viewMode, (m) => {
  if (m === 'preview' && !isPreviewable.value) viewMode.value = 'pretty'
})

// ---------- 查找（Find in Response） ----------
const findOpen = ref(false)
const query = ref('')
const activeMatch = ref(0)
/** JSON 树上报的匹配总数（树视图的权威计数）。 */
const treeTotal = ref(0)

/** JSON 树是否可见（查找对其生效；行视图走本地计数）。 */
const treeVisible = computed(
  () => activeTab.value === 'body' && viewMode.value === 'pretty' && isJson.value && !bodyTooLarge.value,
)

/** 行视图（rp-lines）当前渲染的行；仅按实际可见行计数，保证大响应不卡顿。 */
const searchLines = computed(() => {
  if (!query.value) return []
  if (viewMode.value === 'raw') return shownRawLines.value
  if (viewMode.value === 'preview') return isHtml.value ? [] : shownRawLines.value
  return shownPrettyLines.value
})

function countIn(text: string): number {
  const ql = query.value.toLowerCase()
  let n = 0
  let from = 0
  for (;;) {
    const idx = text.toLowerCase().indexOf(ql, from)
    if (idx === -1) break
    n += 1
    from = idx + ql.length
  }
  return n
}

const textTotal = computed(() => searchLines.value.reduce((n, ln) => n + countIn(ln), 0))

const total = computed(() =>
  treeVisible.value ? treeTotal.value : textTotal.value,
)

watch(query, () => {
  activeMatch.value = 0
})

watch(total, (t) => {
  if (t === 0) activeMatch.value = 0
  else if (activeMatch.value >= t) activeMatch.value = t - 1
})

/** 行视图文本高亮：转义原文并用 <mark> 包裹所有匹配。 */
function highlightText(raw: string, q: string): string {
  if (!q) return escapeHtml(raw)
  const lower = raw.toLowerCase()
  const ql = q.toLowerCase()
  let out = ''
  let from = 0
  for (;;) {
    const idx = lower.indexOf(ql, from)
    if (idx === -1) {
      out += escapeHtml(raw.slice(from))
      return out
    }
    out += escapeHtml(raw.slice(from, idx))
    out += `<mark class="rp-find-mark">${escapeHtml(raw.slice(idx, idx + q.length))}</mark>`
    from = idx + q.length
  }
}

/** Pretty 行视图：JSON 语法高亮 + 查找标记（与请求 Body 编辑器共用同一主题，见 utils/highlight.ts）。 */
function highlightPrettyText(raw: string, q: string): string {
  return highlightJSONText(raw, q)
}

function nextMatch(): void {
  if (!total.value) return
  activeMatch.value = (activeMatch.value + 1) % total.value
}

function prevMatch(): void {
  if (!total.value) return
  activeMatch.value = (activeMatch.value - 1 + total.value) % total.value
}

function closeFind(): void {
  findOpen.value = false
  query.value = ''
  activeMatch.value = 0
  treeTotal.value = 0
}

function toggleFind(): void {
  if (findOpen.value) {
    closeFind()
  } else {
    findOpen.value = true
  }
}

/** ⌘F / Ctrl+F 打开查找（输入框内不拦截）。 */
function onWindowKeydown(e: KeyboardEvent): void {
  if (!(e.metaKey || e.ctrlKey) || e.key.toLowerCase() !== 'f') return
  if (activeTab.value !== 'body' || !props.response.body.trim()) return
  const target = e.target as HTMLElement | null
  if (
    target &&
    (target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable)
  ) {
    return
  }
  e.preventDefault()
  findOpen.value = true
}

const treeRef = ref<{ expandAll: () => void; collapseAll: () => void } | null>(null)

/** 展开/收起全部合并为单切换按钮：记录当前树状态决定动作与图标方向。 */
const treeExpanded = ref(false)
function toggleTreeAll(): void {
  if (treeExpanded.value) {
    treeRef.value?.collapseAll()
    treeExpanded.value = false
  } else {
    treeRef.value?.expandAll()
    treeExpanded.value = true
  }
}

// ---------- 折叠 ----------
/** 折叠状态：只保留状态栏，正文区整体收起（面板随之收缩，剩余空间由响应区布局接管）。 */
const collapsed = ref(false)

function toggleCollapsed(): void {
  collapsed.value = !collapsed.value
}

onMounted(() => window.addEventListener('keydown', onWindowKeydown))
onUnmounted(() => window.removeEventListener('keydown', onWindowKeydown))
</script>

<template>
  <div class="rp" :class="[`tone-${tone}`, { collapsed }]">
    <div class="rp-toolbar">
      <span class="rp-status">
        <Icon name="dot" :size="8" /> {{ statusText }}
      </span>
      <span class="rp-sep"></span>
      <span class="rp-meta">
        <span class="rp-meta-label">耗时</span>
        <span class="rp-meta-value"><Icon name="clock" :size="12" /> {{ formatDuration(response.duration_ms) }}</span>
      </span>
      <span class="rp-sep"></span>
      <span class="rp-meta">
        <span class="rp-meta-label">大小</span>
        <span class="rp-meta-value"><Icon name="package" :size="12" /> {{ sizeText }}</span>
      </span>
      <span v-if="response.content_type" class="rp-sep"></span>
      <span v-if="response.content_type" class="rp-type">{{ response.content_type }}</span>
      <span v-if="response.truncated" class="rp-truncated" title="后端已截断过长的响应正文">已截断</span>

      <Tabs v-model="activeTab" :tabs="responseTabs" size="sm" class="rp-inline-tabs" />
      <span class="rp-toolbar-spacer"></span>

      <SegmentedControl
        v-if="activeTab === 'body'"
        class="rp-mode-seg"
        :model-value="viewMode"
        :options="MODE_OPTIONS"
        size="sm"
        @update:model-value="viewMode = $event as ViewMode"
      />
      <span class="rp-actions">
        <Tooltip content="在响应中查找 (⌘F)" placement="bottom">
          <button
            class="rp-icon-btn"
            type="button"
            :class="{ active: findOpen }"
            @click="toggleFind"
          >
            <Icon name="search" :size="13" />
          </button>
        </Tooltip>
        <Tooltip v-if="treeVisible" :content="treeExpanded ? '收起全部节点' : '展开全部节点'" placement="bottom">
          <button class="rp-icon-btn" type="button" @click="toggleTreeAll">
            <Icon :name="treeExpanded ? 'chevron-up' : 'chevron-down'" :size="13" />
          </button>
        </Tooltip>
        <Tooltip content="将当前响应保存为示例" placement="bottom">
          <button class="rp-icon-btn" type="button" @click="emit('saveExample')">
            <Icon name="save" :size="13" />
          </button>
        </Tooltip>
        <Tooltip content="复制响应正文" placement="bottom">
          <button class="rp-icon-btn" type="button" @click="copyBody">
            <Icon name="copy" :size="13" />
          </button>
        </Tooltip>
        <Tooltip :content="collapsed ? '展开响应区' : '折叠响应区'" placement="bottom">
          <button class="rp-icon-btn" type="button" @click="toggleCollapsed">
            <Icon :name="collapsed ? 'chevron-down' : 'chevron-up'" :size="13" />
          </button>
        </Tooltip>
      </span>
    </div>

    <div v-show="!collapsed" v-if="activeTab === 'body'" class="rp-body">
      <FindBar
        v-if="findOpen && response.body.trim()"
        v-model:query="query"
        :index="activeMatch"
        :total="total"
        @prev="prevMatch"
        @next="nextMatch"
        @close="closeFind"
      />
      <div class="rp-scroll">
        <p v-if="bodyTooLarge" class="rp-note">
          响应超过 1 MB，已按原始文本显示（跳过 JSON 解析与树形渲染以保证流畅）
        </p>
        <p v-if="!response.body.trim()" class="rp-empty">响应正文为空</p>
        <JsonTree
          v-else-if="viewMode === 'pretty' && isJson"
          ref="treeRef"
          :data="parsed"
          :query="treeVisible ? query : ''"
          :active-match="activeMatch"
          @match-count="treeTotal = $event"
        />
        <div v-else-if="viewMode === 'pretty'" class="rp-lines">
          <div v-for="(ln, i) in shownPrettyLines" :key="i" class="rp-line">
            <span class="rp-line-gutter">{{ i + 1 }}</span>
            <span class="rp-line-text" v-html="highlightPrettyText(ln, query)"></span>
          </div>
          <button
            v-if="hasMorePretty"
            class="rp-more"
            type="button"
            @click="showMoreLines"
          >
            显示更多（{{ visibleLines }} / {{ prettyLines.length }} 行）
          </button>
        </div>
        <div v-else-if="viewMode === 'raw'" class="rp-lines">
          <div v-for="(ln, i) in shownRawLines" :key="i" class="rp-line">
            <span class="rp-line-gutter">{{ i + 1 }}</span>
            <span class="rp-line-text" v-html="highlightText(ln, query)"></span>
          </div>
          <button v-if="hasMoreRaw" class="rp-more" type="button" @click="showMoreLines">
            显示更多（{{ visibleLines }} / {{ rawLines.length }} 行）
          </button>
        </div>
        <iframe
          v-else-if="isHtml"
          class="rp-frame"
          sandbox="allow-same-origin"
          :srcdoc="response.body"
          title="响应预览"
        ></iframe>
        <img
          v-else-if="isImage"
          class="rp-frame rp-preview-img"
          :src="`data:${response.content_type};base64,${response.body}`"
          alt="响应图片预览"
        />
        <div v-else class="rp-preview-note">该文件类型不支持内嵌预览，请切换到「原始」视图查看</div>
      </div>
    </div>

    <div v-show="!collapsed" v-else-if="activeTab === 'headers'" class="rp-scroll">
      <div v-for="(h, i) in headerRows" :key="i" class="rp-header-row">
        <span class="rp-header-key">{{ h.k }}</span>
        <span class="rp-header-val">{{ h.v }}</span>
      </div>
      <p v-if="!headerRows.length" class="rp-empty">无响应头</p>
    </div>

    <div v-show="!collapsed" v-else class="rp-scroll">
      <div v-for="(c, i) in cookies" :key="i" class="rp-cookie">
        <div class="rp-cookie-top">
          <span class="rp-cookie-name">{{ c.name }}</span>
          <span class="rp-cookie-value">{{ c.value }}</span>
          <span class="rp-cookie-flags">
            <span v-if="c.secure" class="rp-flag">Secure</span>
            <span v-if="c.httpOnly" class="rp-flag">HttpOnly</span>
            <span v-if="c.sameSite" class="rp-flag">{{ c.sameSite }}</span>
          </span>
        </div>
        <div v-if="c.domain || c.path || c.expires" class="rp-cookie-meta">
          <span v-if="c.domain">域：{{ c.domain }}</span>
          <span v-if="c.path">路径：{{ c.path }}</span>
          <span v-if="c.expires">过期：{{ c.expires }}</span>
        </div>
      </div>
      <p v-if="!cookies.length" class="rp-empty">响应未携带 Set-Cookie</p>
    </div>
  </div>
</template>

<style scoped>
.rp {
  border: 1px solid var(--border-strong);
  /* 顶边无边框：与请求区之间仅由分割条分隔（Single Border Architecture） */
  border-top: none;
  border-radius: var(--radius);
  background: var(--bg-card);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}
.rp.tone-err {
  border-color: var(--danger-border);
  border-top: none;
}
/* 折叠时只保留状态栏，面板收缩为内容高度 */
.rp.collapsed {
  height: auto;
}

/* ---- 单行工具栏：状态指标 + 页签 + 模式 + 操作 ---- */
.rp-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 36px;
  padding: 4px 10px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
  flex-shrink: 0;
  overflow: hidden;
}

.rp-inline-tabs {
  flex-shrink: 0;
}
.rp-inline-tabs :deep(.tabs) {
  border-bottom: none;
}
.rp-inline-tabs :deep(.tab) {
  height: 28px;
}

.rp-toolbar-spacer {
  flex: 1 1 auto;
  min-width: 8px;
}

.rp-mode-seg {
  flex-shrink: 0;
}

.rp-status {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 6px;
  font-weight: 700;
  font-size: 12px;
  font-family: var(--font-mono);
  line-height: 1.4;
  letter-spacing: 0.2px;
}
.rp.tone-ok .rp-status {
  background: var(--success);
  color: #fff;
  box-shadow: 0 2px 10px rgba(34, 197, 94, 0.35);
}
.rp.tone-warn .rp-status {
  background: #b45309;
  color: #fff;
  box-shadow: 0 2px 10px rgba(245, 158, 11, 0.3);
}
.rp.tone-err .rp-status {
  background: var(--danger);
  color: #fff;
  box-shadow: 0 2px 10px rgba(239, 68, 68, 0.35);
}

/* 指标竖线分隔（border-white/10 h-3） */
.rp-sep {
  flex-shrink: 0;
  width: 1px;
  height: 14px;
  background: rgba(255, 255, 255, 0.12);
}

.rp-meta {
  flex-shrink: 0;
  display: inline-flex;
  align-items: baseline;
  gap: 6px;
  white-space: nowrap;
}

.rp-meta-label {
  font-size: 11px;
  color: var(--text-3);
}

.rp-meta-value {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--text-1);
}
.rp-meta-value svg {
  color: var(--accent);
  opacity: 0.9;
}

.rp-type {
  min-width: 0;
  font-size: 11.5px;
  color: var(--text-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.rp-truncated {
  flex-shrink: 0;
  padding: 1px 8px;
  border-radius: 999px;
  font-size: 10.5px;
  font-weight: 600;
  color: var(--warning);
  background: var(--warning-tint);
}

/* 最右操作区（纯图标按钮，自带 Tooltip） */
.rp-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 4px;
  margin-left: 6px;
}
.rp-icon-btn {
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
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease);
}
.rp-icon-btn:hover {
  background: var(--bg-hover);
  color: var(--text-1);
}
.rp-icon-btn svg {
  color: var(--accent);
}
.rp-icon-btn.active {
  color: var(--accent);
  background: var(--accent-tint, var(--bg-hover));
}

/* ---- 查找 ---- */
:deep(.rp-find-mark) {
  background: var(--accent-tint, rgba(99, 102, 241, 0.25));
  color: inherit;
  border-radius: 2px;
  padding: 0 1px;
}

/* ---- 正文区 ---- */
.rp-body {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.rp-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 0;
}

.rp-lines {
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.55;
}

.rp-line {
  display: flex;
  align-items: flex-start;
  min-width: 0;
  white-space: pre-wrap;
  word-break: break-all;
  color: var(--text-1);
}

.rp-line-gutter {
  flex-shrink: 0;
  width: 38px;
  text-align: right;
  padding-right: 10px;
  user-select: none;
  color: var(--tok-gutter);
  font-size: 11px;
}

.rp-line-text {
  min-width: 0;
  flex: 1;
}

.rp-frame {
  display: block;
  width: 100%;
  height: 100%;
  border: none;
  background: var(--bg-panel);
}

.rp-preview-img {
  object-fit: contain;
  padding: 8px;
}

.rp-preview-note {
  margin: 0;
  padding: 14px 16px;
  font-size: 12px;
  color: var(--text-3);
}

.rp-empty {
  margin: 0;
  padding: 14px 16px;
  font-size: 12px;
  color: var(--text-3);
}

.rp-note {
  margin: 0;
  padding: 6px 12px 0;
  font-size: 11.5px;
  color: var(--warning);
}

.rp-more {
  display: block;
  width: 100%;
  padding: 8px;
  border: none;
  border-top: 1px dashed var(--border);
  background: none;
  font-family: inherit;
  font-size: 11.5px;
  color: var(--accent);
  cursor: pointer;
}
.rp-more:hover {
  opacity: 0.8;
}

/* ---- Headers ---- */
.rp-header-row {
  display: grid;
  grid-template-columns: minmax(120px, 260px) 1fr;
  gap: 10px;
  align-items: baseline;
  padding: 5px 12px;
  border-bottom: 1px dashed var(--border);
  font-size: 11.5px;
}
.rp-header-row:last-child {
  border-bottom: none;
}

.rp-header-key {
  font-weight: 600;
  color: var(--text-1);
  word-break: break-all;
  overflow-wrap: anywhere;
}

.rp-header-val {
  color: var(--text-2);
  word-break: break-all;
  overflow-wrap: anywhere;
}

/* ---- Cookies ---- */
.rp-cookie {
  margin: 0 12px 8px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.rp-cookie-top {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}

.rp-cookie-name {
  flex-shrink: 0;
  font-weight: 700;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-1);
}

.rp-cookie-value {
  flex: 1;
  min-width: 0;
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-2);
  word-break: break-all;
}

.rp-cookie-flags {
  flex-shrink: 0;
  display: inline-flex;
  gap: 4px;
}

.rp-flag {
  padding: 1px 6px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 600;
  color: var(--info);
  background: var(--info-tint, var(--accent-tint));
}

.rp-cookie-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  font-size: 11px;
  color: var(--text-3);
}
</style>

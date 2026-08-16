<script setup lang="ts">
/**
 * ResponsePanel：响应面板。
 * - 顶栏：状态 pill（2xx 实心绿 / 3xx 琥珀 / 4xx-5xx 红，`200 OK`）+ 指标（⏱ 耗时 / 📦 大小，竖线分隔）+ 类型；
 * - 工具栏：Body/Headers/Cookies 标签 + 格式化/原始/预览 分段切换（右） + 保存为示例 / 复制响应（最右）；
 * - 主体：JSON → 可折叠树形查看器（行号 + VS Code 深色语法着色）；文本 → 行号代码视图；HTML → 沙箱预览。
 */
import { computed, ref, watch } from 'vue'
import { useToast } from '../composables/useToast'
import { formatBytes, formatDuration } from '../utils/format'
import Icon from './ui/Icon.vue'
import JsonTree from './JsonTree.vue'
import SegmentedControl, { type SegmentOption } from './ui/SegmentedControl.vue'
import Tabs, { type TabItem } from './ui/Tabs.vue'
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
  if (parsed.value !== null) return JSON.stringify(parsed.value, null, 2)
  return props.response.body
})

const isHtml = computed(() => props.response.content_type.toLowerCase().includes('html'))

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

const MODE_OPTIONS: SegmentOption[] = [
  { value: 'pretty', label: '格式化', icon: 'list' },
  { value: 'raw', label: '原始', icon: 'code' },
  { value: 'preview', label: '预览', icon: 'eye' },
]
</script>

<template>
  <div class="rp" :class="`tone-${tone}`">
    <div class="rp-head">
      <span class="rp-status">
        <Icon name="dot" :size="8" /> {{ statusText }}
      </span>
      <span class="rp-sep"></span>
      <span class="rp-meta"><Icon name="clock" :size="13" /> {{ formatDuration(response.duration_ms) }}</span>
      <span class="rp-sep"></span>
      <span class="rp-meta"><Icon name="package" :size="13" /> {{ sizeText }}</span>
      <span v-if="response.content_type" class="rp-sep"></span>
      <span v-if="response.content_type" class="rp-type">{{ response.content_type }}</span>
      <span v-if="response.truncated" class="rp-truncated" title="后端已截断过长的响应正文">已截断</span>
    </div>

    <div class="rp-tabs">
      <Tabs v-model="activeTab" :tabs="responseTabs" size="sm" />
      <span class="rp-tabs-spacer"></span>
      <SegmentedControl
        v-if="activeTab === 'body'"
        class="rp-mode-seg"
        :model-value="viewMode"
        :options="MODE_OPTIONS"
        size="sm"
        @update:model-value="viewMode = $event as ViewMode"
      />
      <span class="rp-actions">
        <button class="rf-btn rf-btn-sm" type="button" title="将当前响应保存为示例" @click="emit('saveExample')">
          <Icon name="save" :size="13" /> 保存为示例
        </button>
        <button class="rf-btn rf-btn-sm" type="button" title="复制响应正文" @click="copyBody">
          <Icon name="copy" :size="13" /> 复制响应
        </button>
      </span>
    </div>

    <div v-if="activeTab === 'body'" class="rp-scroll">
      <p v-if="bodyTooLarge" class="rp-note">
        响应超过 1 MB，已按原始文本显示（跳过 JSON 解析与树形渲染以保证流畅）
      </p>
      <p v-if="!response.body.trim()" class="rp-empty">响应正文为空</p>
      <JsonTree v-else-if="viewMode === 'pretty' && isJson" :data="parsed" />
      <div v-else-if="viewMode === 'pretty'" class="rp-lines">
        <div v-for="(ln, i) in shownPrettyLines" :key="i" class="rp-line">
          <span class="rp-line-gutter">{{ i + 1 }}</span>
          <span class="rp-line-text">{{ ln }}</span>
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
          <span class="rp-line-text">{{ ln }}</span>
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
      <div v-else class="rp-lines">
        <div v-for="(ln, i) in shownRawLines" :key="i" class="rp-line">
          <span class="rp-line-gutter">{{ i + 1 }}</span>
          <span class="rp-line-text">{{ ln }}</span>
        </div>
        <button v-if="hasMoreRaw" class="rp-more" type="button" @click="showMoreLines">
          显示更多（{{ visibleLines }} / {{ rawLines.length }} 行）
        </button>
      </div>
    </div>

    <div v-else-if="activeTab === 'headers'" class="rp-scroll">
      <div v-for="(h, i) in headerRows" :key="i" class="rp-header-row">
        <span class="rp-header-key">{{ h.k }}</span>
        <span class="rp-header-val">{{ h.v }}</span>
      </div>
      <p v-if="!headerRows.length" class="rp-empty">无响应头</p>
    </div>

    <div v-else class="rp-scroll">
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
  border-radius: var(--radius);
  background: var(--bg-card);
  overflow: hidden;
}
.rp.tone-err {
  border-color: var(--danger-border);
}

/* ---- 顶栏 ---- */
.rp-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
  overflow: hidden;
}

.rp-status {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 999px;
  font-weight: 700;
  font-size: 12px;
  font-family: var(--font-mono);
}
.rp.tone-ok .rp-status {
  background: var(--success);
  color: #fff;
  box-shadow: 0 2px 10px rgba(34, 197, 94, 0.35);
}
.rp.tone-warn .rp-status {
  background: var(--warning-tint);
  color: var(--warning);
}
.rp.tone-err .rp-status {
  background: var(--danger-tint);
  color: var(--danger);
}

/* 指标竖线分隔（border-white/10 h-3） */
.rp-sep {
  flex-shrink: 0;
  width: 1px;
  height: 12px;
  background: rgba(255, 255, 255, 0.1);
}

.rp-meta {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  font-family: var(--font-mono);
  color: var(--text-2);
  white-space: nowrap;
}
.rp-meta svg {
  opacity: 0.75;
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

/* ---- 工具栏：标签 + 视图分段 + 操作 ---- */
.rp-tabs {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 10px 0 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
  overflow: hidden;
}

.rp-tabs-spacer {
  flex: 1 1 auto;
}

.rp-mode-seg {
  flex-shrink: 0;
}

/* 最右操作区（与视图分段控件留出清晰间隔） */
.rp-actions {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-left: 18px;
}
.rp-actions .rf-btn {
  color: var(--text-2);
}
.rp-actions .rf-btn svg {
  color: var(--accent);
}

/* ---- 正文区 ---- */
.rp-scroll {
  max-height: 420px;
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
  color: var(--text-3);
  font-size: 11px;
  opacity: 0.7;
}

.rp-line-text {
  min-width: 0;
  flex: 1;
}

.rp-frame {
  display: block;
  width: 100%;
  height: 380px;
  border: none;
  background: var(--bg-panel);
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

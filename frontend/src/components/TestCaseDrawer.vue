<script setup lang="ts">
/**
 * TestCaseDrawer：测试用例详情 / 编辑抽屉（右侧滑出，原地运行）。
 * - 三区分区：用例基本信息 / 请求区（Params · Headers · Body 微型 Tab）+ 响应区（拖拽分割）；
 * - Method + Path 无缝组合输入（Method Badge 下拉 + Path flex-1），Method 联动默认 Tab 与 Content-Type；
 * - Body 与 Response 均使用 CodeMirror 6（JSON 暗黑高亮 / 折叠 / 错误校验 / 只读）；
 * - 请求区与响应区之间可拖拽分割条（双击恢复 50%，双方 min-height 120px）；
 * - 底部操作栏 sticky，左侧「立即运行」、右侧「取消 / 保存修改」。
 */
import { computed, ref, watch } from 'vue'
import type { ExecuteResponse, HttpMethod, KeyValue, TestCase, TestCaseCategory } from '../types/foxApi'
import { TEST_CASE_CATEGORIES, formatDuration, statusToneOf, statusTextOf } from '../utils/testCases'
import CustomSelect from './ui/CustomSelect.vue'
import Icon from './ui/Icon.vue'
import JsonCodeMirror from './JsonCodeMirror.vue'
import KeyValueTable, { type KVRow } from './ui/KeyValueTable.vue'

const props = defineProps<{
  open: boolean
  /** 当前接口 id（request_id），用于保存后回填本地列表。 */
  endpointId: string
  testCase: TestCase | null
  /** 运行回调：返回响应或抛错。由父级（store 或 panel）注入，避免组件耦合执行细节。 */
  onRun: (payload: {
    method: HttpMethod
    urlPath: string
    params: KeyValue[]
    headers: KeyValue[]
    bodyType: string
    bodyContent: string
  }) => Promise<ExecuteResponse | null>
  onSave: (payload: {
    name: string
    category: TestCaseCategory
    method: HttpMethod
    urlPath: string
    params: KeyValue[]
    headers: KeyValue[]
    bodyType: string
    bodyContent: string
  }) => Promise<void>
}>()

const emit = defineEmits<{ 'update:open': [open: boolean] }>()

const name = ref('')
const category = ref<TestCaseCategory>('正向')
const method = ref<HttpMethod>('GET')
const urlPath = ref('')
const params = ref<KeyValue[]>([])
const headers = ref<KeyValue[]>([])
const bodyType = ref('none')
const bodyContent = ref('')

const METHOD_OPTIONS = (['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'] as HttpMethod[]).map(
  (m) => ({ value: m, label: m }),
)
const BODY_OPTIONS = [
  { value: 'none', label: '无 Body' },
  { value: 'json', label: 'JSON' },
  { value: 'form-data', label: 'Form-Data' },
  { value: 'raw', label: 'Raw' },
  { value: 'urlencoded', label: 'URL-Encoded' },
  { value: 'graphql', label: 'GraphQL' },
  { value: 'binary', label: 'Binary' },
].map((o) => ({ value: o.value, label: o.label }))

// ---------- 请求参数 Tab ----------
type ReqTab = 'params' | 'headers' | 'body'
const activeTab = ref<ReqTab>('params')
const REQ_TABS: { key: ReqTab; label: string }[] = [
  { key: 'params', label: 'Params' },
  { key: 'headers', label: 'Headers' },
  { key: 'body', label: 'Body' },
]

const result = ref<ExecuteResponse | null>(null)
const runError = ref('')

// ---------- Method 联动：默认 Tab 与 Content-Type ----------
const BODY_METHODS = ['POST', 'PUT', 'PATCH']
const NO_BODY_METHODS = ['GET', 'DELETE', 'HEAD', 'OPTIONS']

/** POST/PUT/PATCH → Body Tab + Content-Type: application/json；其余 → Params Tab。 */
function applyMethodDefaults(m: HttpMethod): void {
  if (BODY_METHODS.includes(m)) {
    activeTab.value = 'body'
    if (bodyType.value === 'none') bodyType.value = 'json'
    if (!headers.value.some((h) => h.key?.toLowerCase() === 'content-type')) {
      headers.value.push({ key: 'Content-Type', value: 'application/json', enabled: true, description: '' })
    }
  } else {
    activeTab.value = 'params'
  }
}

watch(method, (m) => applyMethodDefaults(m))

// ---------- 打开时同步草稿 ----------
watch(
  () => props.open,
  (open) => {
    if (!open || !props.testCase) return
    name.value = props.testCase.name
    category.value = props.testCase.category
    method.value = props.testCase.method
    urlPath.value = props.testCase.url_path
    params.value = props.testCase.params.map((p) => ({ ...p }))
    headers.value = props.testCase.headers.map((h) => ({ ...h }))
    bodyType.value = props.testCase.body_type
    bodyContent.value = props.testCase.body_content
    activeTab.value = 'params'
    result.value = null
    runError.value = ''
    applyMethodDefaults(method.value)
  },
  { immediate: true },
)

// ---------- 请求参数 Tab ----------
const paramRows = computed({
  get: () => params.value,
  set: (rows: KVRow[]) => {
    params.value = rows as KeyValue[]
  },
})
const headerRows = computed({
  get: () => headers.value,
  set: (rows: KVRow[]) => {
    headers.value = rows as KeyValue[]
  },
})

// ---------- 请求参数 Tab ----------
const bodyLabel = computed(() => BODY_OPTIONS.find((o) => o.value === bodyType.value)?.label ?? bodyType.value)
const bodyEditable = computed(() => bodyType.value !== 'none')

// ---------- 底部操作 ----------
const saving = ref(false)
const running = ref(false)

function payload() {
  return {
    method: method.value,
    urlPath: urlPath.value,
    params: params.value.filter((p) => p.key || p.value),
    headers: headers.value.filter((h) => h.key || h.value),
    bodyType: bodyType.value,
    bodyContent: bodyContent.value,
  }
}

async function save(): Promise<void> {
  if (!props.testCase) return
  if (!name.value.trim() || !urlPath.value.trim()) return
  saving.value = true
  try {
    await props.onSave({ ...payload(), name: name.value.trim(), category: category.value })
    emit('update:open', false)
  } finally {
    saving.value = false
  }
}

async function run(): Promise<void> {
  if (!props.testCase) return
  if (!urlPath.value.trim()) return
  running.value = true
  result.value = null
  runError.value = ''
  try {
    result.value = await props.onRun(payload())
  } catch (err) {
    runError.value = err instanceof Error ? err.message : String(err)
  } finally {
    running.value = false
  }
}

// ---------- 响应展示 ----------
/** 响应 Body：JSON 时美化缩进，非 JSON 原样展示（交由 CodeMirror 只读渲染）。 */
const prettyBody = computed(() => {
  const text = result.value?.body ?? ''
  if (!text) return ''
  try {
    return JSON.stringify(JSON.parse(text), null, 2)
  } catch {
    return text
  }
})

// ---------- Method 联动：默认 Tab 与 Content-Type ----------
const bodyTabHint = computed(() =>
  activeTab.value === 'body' && NO_BODY_METHODS.includes(method.value)
    ? `${method.value} 请求通常不携带 Body`
    : '',
)

const copied = ref(false)

async function copyBody(): Promise<void> {
  if (!result.value?.body) return
  await navigator.clipboard.writeText(result.value.body)
  copied.value = true
  setTimeout(() => (copied.value = false), 1500)
}

/** 大小格式化：<1KB 显示 B，否则 KB（1 位小数）。 */
function sizeText(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`
  return `${(bytes / 1024).toFixed(1)}KB`
}

const resultTone = computed(() => (result.value ? statusToneOf(result.value.status) : 'info'))

/** Body 自动格式化（JSON / GraphQL 可解析时美化缩进）。 */
function formatBody(): void {
  if (!['json', 'graphql'].includes(bodyType.value)) return
  try {
    bodyContent.value = JSON.stringify(JSON.parse(bodyContent.value), null, 2)
  } catch {
    /* 非法 JSON 不动 */
  }
}

const methodClass = computed(() => `m-select-${method.value.toLowerCase()}`)

// ---------- 请求区 / 响应区垂直拖拽分割 ----------
const splitArea = ref<HTMLElement | null>(null)
const bodyEditorRef = ref<InstanceType<typeof JsonCodeMirror> | null>(null)
const respEditorRef = ref<InstanceType<typeof JsonCodeMirror> | null>(null)

/** 请求区占比（0~1），响应区占剩余空间。 */
const reqRatio = ref(0.55)
const MIN_PX = 120
let dragging = false
let startY = 0
let startRatio = 0

/** 请求区百分比（去除浮点尾差，如 55.00000000000001 → 55%）。 */
const reqPct = computed(() => `${parseFloat((reqRatio.value * 100).toFixed(2))}%`)

function measureEditors(): void {
  requestAnimationFrame(() => {
    bodyEditorRef.value?.requestMeasure()
    respEditorRef.value?.requestMeasure()
  })
}

function onSplitterDown(e: MouseEvent): void {
  if (e.button !== 0) return
  e.preventDefault()
  dragging = true
  startY = e.clientY
  startRatio = reqRatio.value
  window.addEventListener('mousemove', onSplitterMove)
  window.addEventListener('mouseup', onSplitterUp)
}

function onSplitterMove(e: MouseEvent): void {
  if (!dragging) return
  const rect = splitArea.value?.getBoundingClientRect()
  if (!rect || rect.height === 0) return
  const min = MIN_PX / rect.height
  const ratio = startRatio + (e.clientY - startY) / rect.height
  reqRatio.value = Math.min(1 - min, Math.max(min, ratio))
  measureEditors()
}

function onSplitterUp(): void {
  dragging = false
  window.removeEventListener('mousemove', onSplitterMove)
  window.removeEventListener('mouseup', onSplitterUp)
}

function onSplitterDblClick(): void {
  reqRatio.value = 0.5
  measureEditors()
}
</script>

<template>
  <Teleport to="body">
    <Transition name="drw">
      <div v-if="open" class="drw-mask" @mousedown.self="emit('update:open', false)">
        <aside class="drw" role="dialog" aria-modal="true">
          <header class="drw-head">
            <h3 class="drw-title">用例详情 / 编辑</h3>
            <button class="drw-close" type="button" title="关闭" @click="emit('update:open', false)">
              <Icon name="x" :size="15" />
            </button>
          </header>

          <div class="drw-body">
            <!-- ① 用例基本信息 -->
            <section class="drw-sec">
              <h4 class="drw-sec-title">用例基本信息</h4>
              <div class="drw-grid3">
                <label class="drw-field drw-span2">
                  <span class="drw-label">用例名称</span>
                  <input v-model="name" class="drw-input" type="text" spellcheck="false" placeholder="如：内部划转-SGB" />
                </label>
                <label class="drw-field">
                  <span class="drw-label">分组</span>
                  <CustomSelect
                    :model-value="category"
                    :options="TEST_CASE_CATEGORIES.map((c) => ({ value: c, label: c }))"
                    @update:model-value="category = String($event) as TestCaseCategory"
                  />
                </label>
              </div>

              <!-- Method + Path 组合输入 -->
              <label class="drw-field">
                <span class="drw-label">请求地址</span>
                <div class="drw-url-group">
                  <CustomSelect
                    class="drw-method"
                    :class="methodClass"
                    :model-value="method"
                    :options="METHOD_OPTIONS"
                    @update:model-value="method = String($event) as HttpMethod"
                  >
                    <template #display="{ label }">
                      <span :class="`drw-method-badge m-select-${label.toLowerCase()}`">{{ label }}</span>
                    </template>
                  </CustomSelect>
                  <input
                    v-model="urlPath"
                    class="drw-input drw-mono drw-path-input"
                    type="text"
                    spellcheck="false"
                    placeholder="/funds/transfer 或 https://…"
                  />
                </div>
              </label>
            </section>

            <!-- ②③ 请求区 / 响应区：垂直拖拽分割 -->
            <div ref="splitArea" class="drw-split">
              <section class="drw-sec drw-req-sec" :style="{ flexBasis: reqPct }">
                <div class="drw-sec-head">
                  <h4 class="drw-sec-title">请求参数配置</h4>
                  <div class="drw-tabs" role="tablist">
                    <button
                      v-for="t in REQ_TABS"
                      :key="t.key"
                      class="drw-tab"
                      :class="{ active: activeTab === t.key }"
                      type="button"
                      role="tab"
                      @click="activeTab = t.key"
                    >
                      {{ t.label }}
                    </button>
                  </div>
                </div>

                <KeyValueTable
                  v-if="activeTab === 'params'"
                  v-model="paramRows"
                  :column-widths="['30%', '50%', '20%']"
                />
                <KeyValueTable
                  v-else-if="activeTab === 'headers'"
                  v-model="headerRows"
                  :column-widths="['30%', '50%', '20%']"
                />
                <div v-else class="drw-body-pane">
                  <div class="drw-body-type">
                    <span class="drw-label">Body 类型</span>
                    <CustomSelect
                      :model-value="bodyType"
                      :options="BODY_OPTIONS"
                      @update:model-value="bodyType = String($event)"
                    />
                    <button
                      v-if="bodyEditable"
                      class="drw-fmt-btn"
                      type="button"
                      title="自动格式化 JSON"
                      @click="formatBody"
                    >
                      格式化
                    </button>
                    <span class="drw-body-hint">{{ bodyLabel }}</span>
                  </div>
                  <div v-if="bodyEditable" class="drw-cm-wrap">
                    <JsonCodeMirror
                      ref="bodyEditorRef"
                      :model-value="bodyContent"
                      :placeholder-text='bodyType === "json" ? "{\"key\": \"value\"}" : "请求内容…"'
                      @update:model-value="bodyContent = $event"
                    />
                  </div>
                  <p v-else class="drw-none">当前类型无请求内容</p>
                  <p v-if="bodyTabHint" class="drw-body-warn">{{ bodyTabHint }}</p>
                </div>
              </section>

              <div
                class="drw-splitter"
                role="separator"
                aria-orientation="horizontal"
                title="拖拽调整请求区 / 响应区比例，双击恢复 50%"
                @mousedown="onSplitterDown"
                @dblclick="onSplitterDblClick"
              ></div>

              <section class="drw-sec drw-resp-sec">
                <h4 class="drw-sec-title">运行响应结果</h4>

                <div v-if="running" class="drw-result drw-result-idle">
                  <span class="drw-spinner"></span>
                  <span class="drw-result-msg">请求发送中…</span>
                </div>
                <div v-else-if="runError" class="drw-result">
                  <div class="drw-result-bar">
                    <span class="drw-badge err">Failed</span>
                    <span class="drw-result-msg">请求失败：{{ runError }}</span>
                  </div>
                </div>
                <div v-else-if="result" class="drw-result drw-result-main">
                  <div class="drw-result-bar">
                    <span class="drw-badge" :class="`tone-${resultTone}`">
                      {{ result.status }} {{ statusTextOf(result.status) }}
                    </span>
                    <span class="drw-meta">{{ formatDuration(result.duration_ms) }}</span>
                    <span class="drw-meta">{{ sizeText(result.size_bytes) }}</span>
                    <span class="drw-copy">
                      <button
                        class="drw-copy-btn"
                        type="button"
                        title="复制 Response Body"
                        @click="copyBody"
                      >
                        <Icon :name="copied ? 'check' : 'copy'" :size="12" />
                        {{ copied ? '已复制' : '复制' }}
                      </button>
                    </span>
                  </div>
                  <div v-if="prettyBody" class="drw-cm-wrap">
                    <JsonCodeMirror
                      ref="respEditorRef"
                      :model-value="prettyBody"
                      readonly
                    />
                  </div>
                  <p v-else class="drw-none">（空响应体）</p>
                </div>
                <div v-else class="drw-result drw-result-idle">
                  <span class="drw-result-msg">尚无响应数据，点击下方「立即运行」发起请求</span>
                </div>
              </section>
            </div>
          </div>

          <footer class="drw-foot">
            <button
              class="rf-btn drw-run-btn"
              type="button"
              :disabled="running || !urlPath.trim()"
              @click="run"
            >
              <span v-if="running" class="drw-spinner"></span>
              <Icon v-else name="play" :size="13" /> {{ running ? '运行中…' : '立即运行' }}
            </button>
            <span class="drw-spacer"></span>
            <button class="rf-btn" type="button" :disabled="saving || running" @click="emit('update:open', false)">
              取消
            </button>
            <button
              class="rf-btn rf-btn-primary"
              type="button"
              :disabled="saving || running || !name.trim() || !urlPath.trim()"
              @click="save"
            >
              <Icon name="save" :size="13" /> 保存修改
            </button>
          </footer>
        </aside>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.drw-mask {
  position: fixed;
  inset: 0;
  z-index: 120;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(2px);
}

.drw {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 720px;
  max-width: 94vw;
  display: flex;
  flex-direction: column;
  background: var(--bg-2);
  border-left: 1px solid var(--border);
  box-shadow: -12px 0 40px rgba(0, 0, 0, 0.35);
}

.drw-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.drw-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1);
}

.drw-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: none;
  border-radius: 6px;
  background: none;
  color: var(--text-3);
  cursor: pointer;
  transition:
    color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.drw-close:hover {
  color: var(--text-1);
  background: var(--bg-hover);
}

.drw-body {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px 18px 20px;
}

/* 请求区 + 分割条 + 响应区（垂直拖拽分割） */
.drw-split {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.drw-splitter {
  flex: 0 0 6px;
  border-radius: 3px;
  background: color-mix(in srgb, var(--text-3) 25%, transparent);
  cursor: row-resize;
  transition: background 0.15s ease;
  user-select: none;
}
.drw-splitter:hover,
.drw-splitter:active {
  background: rgba(168, 85, 247, 0.55);
}

/* ---------- 分区卡片 ---------- */
.drw-sec {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-card, var(--bg-1));
}

/* 请求区：高度由分割比例控制，内容超限时区内滚动 */
.drw-req-sec {
  flex: 0 0 auto;
  min-height: 120px;
  overflow: auto;
}

.drw-sec-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.drw-sec-title {
  margin: 0;
  font-size: 11.5px;
  font-weight: 600;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  color: var(--text-3);
}

/* ---------- 基本信息 ---------- */
.drw-grid3 {
  display: grid;
  grid-template-columns: 2fr 1fr;
  gap: 12px;
}

.drw-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.drw-label {
  font-size: 12px;
  color: var(--text-2);
}

.drw-input {
  width: 100%;
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  font-family: inherit;
  font-size: 13px;
  color: var(--text-1);
  background: var(--bg-1);
  outline: none;
  transition:
    border-color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.drw-input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint, rgba(168, 85, 247, 0.18));
}

.drw-mono {
  font-family: var(--font-mono);
  font-size: 12px;
}

/* ---------- Method + Path 组合 ---------- */
.drw-url-group {
  display: flex;
  align-items: stretch;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-1);
  transition:
    border-color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}
.drw-url-group:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint, rgba(168, 85, 247, 0.18));
}

.drw-method {
  flex-shrink: 0;
  border: none;
  background: transparent;
  border-radius: 0;
}

.drw-method-badge {
  display: inline-flex;
  align-items: center;
  height: 20px;
  padding: 0 8px;
  border-radius: 5px;
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.3px;
  background: color-mix(in srgb, currentColor 14%, transparent);
}

.drw-path-input {
  flex: 1;
  min-width: 0;
  border: none;
  border-left: 1px solid var(--border);
  border-radius: 0;
  background: transparent;
}
.drw-path-input:focus {
  box-shadow: none;
}

/* ---------- 请求参数 Tab ---------- */
.drw-tabs {
  display: flex;
  gap: 2px;
  padding: 2px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--bg-2);
}

.drw-tab {
  padding: 4px 12px;
  border: none;
  border-radius: 5px;
  background: none;
  font-family: inherit;
  font-size: 11.5px;
  color: var(--text-3);
  cursor: pointer;
  transition:
    color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.drw-tab:hover {
  color: var(--text-1);
}
.drw-tab.active {
  color: var(--accent);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-sm);
}

.drw-body-pane {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.drw-body-type {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
}

.drw-body-hint {
  font-size: 12px;
  color: var(--text-3);
}

.drw-cm-wrap {
  flex: 1;
  min-height: 0;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
  background: var(--bg-2);
}

.drw-body-warn {
  margin: 0;
  font-size: 11.5px;
  color: var(--warning);
}

.drw-fmt-btn {
  margin-left: auto;
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: none;
  font-family: inherit;
  font-size: 11.5px;
  color: var(--text-2);
  cursor: pointer;
  transition:
    color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.drw-fmt-btn:hover {
  color: var(--text-1);
  background: var(--bg-hover);
}

.drw-none {
  margin: 0;
  padding: 8px 2px;
  font-size: 12.5px;
  color: var(--text-3);
}

/* ---------- 响应区 ---------- */
.drw-resp-sec {
  flex: 1 1 0;
  min-height: 120px;
  overflow: hidden;
  gap: 10px;
}

.drw-result {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* 有结果时：撑满响应区，CodeMirror 只读预览随高度自适应 */
.drw-result-main {
  flex: 1;
  min-height: 0;
}

.drw-result-idle {
  flex-direction: row;
  align-items: center;
  gap: 8px;
  padding: 6px 2px;
  min-height: 0;
}

.drw-result-bar {
  display: flex;
  align-items: center;
  gap: 10px;
}

.drw-badge {
  flex-shrink: 0;
  padding: 2px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
  font-family: var(--font-mono);
  border: 1px solid transparent;
}
.drw-badge.tone-ok {
  color: var(--ok);
  background: color-mix(in srgb, var(--ok) 10%, transparent);
  border-color: color-mix(in srgb, var(--ok) 22%, transparent);
}
.drw-badge.tone-warn {
  color: var(--warning);
  background: color-mix(in srgb, var(--warning) 10%, transparent);
  border-color: color-mix(in srgb, var(--warning) 22%, transparent);
}
.drw-badge.tone-err {
  color: var(--danger);
  background: color-mix(in srgb, var(--danger) 10%, transparent);
  border-color: color-mix(in srgb, var(--danger) 22%, transparent);
}
.drw-badge.tone-info {
  color: var(--rf-info, #38bdf8);
  background: color-mix(in srgb, var(--rf-info, #38bdf8) 10%, transparent);
  border-color: color-mix(in srgb, var(--rf-info, #38bdf8) 22%, transparent);
}

.drw-meta {
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-3);
}

.drw-copy {
  margin-left: auto;
}

.drw-copy-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 8px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: none;
  font-family: inherit;
  font-size: 11.5px;
  color: var(--text-2);
  cursor: pointer;
  transition:
    color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.drw-copy-btn:hover {
  color: var(--text-1);
  background: var(--bg-hover);
}

.drw-result-msg {
  font-size: 12px;
  color: var(--text-3);
}

.drw-spinner {
  width: 13px;
  height: 13px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: drw-spin 0.7s linear infinite;
}

@keyframes drw-spin {
  to {
    transform: rotate(360deg);
  }
}

/* ---------- Sticky Footer ---------- */
.drw-foot {
  position: sticky;
  bottom: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 18px;
  border-top: 1px solid var(--border);
  background: color-mix(in srgb, var(--bg-2) 90%, transparent);
  backdrop-filter: blur(8px);
  flex-shrink: 0;
  z-index: 2;
}

.drw-run-btn {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 55%, transparent);
}
.drw-run-btn:hover:not(:disabled) {
  background: var(--accent-tint, rgba(168, 85, 247, 0.14));
}

.drw-spacer {
  flex: 1;
}

.drw-mask-enter-active,
.drw-mask-leave-active {
  transition: opacity 0.18s var(--ease);
}
.drw-mask-enter-from,
.drw-mask-leave-to {
  opacity: 0;
}
.drw-mask-enter-active .drw,
.drw-mask-leave-active .drw {
  transition: transform 0.22s var(--ease);
}
.drw-mask-enter-from .drw,
.drw-mask-leave-to .drw {
  transform: translateX(100%);
}
</style>
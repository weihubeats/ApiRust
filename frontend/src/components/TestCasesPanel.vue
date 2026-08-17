<script setup lang="ts">
/**
 * TestCasesPanel：Apifox 风格测试用例管理面板。
 * - 顶部分类 Tab（全部 (N) / 正向 / 负向 / 边界值 / 安全性 / 其他），点击过滤；
 * - 右上角：+ 添加用例 / ▶ 全部运行；
 * - 数据表：# | 名称 | 分组 | 运行结果 | 操作（... 菜单：直接运行 / 编辑 / 克隆 / 删除）；
 * - 点击用例行 → 回填调试页（method / path / params / headers / body）并切回「调试」。
 */
import { computed, ref, watch } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { CATEGORY_TONE, TEST_CASE_CATEGORIES, formatDuration, statusTextOf, statusToneOf } from '../utils/testCases'
import type { Endpoint, TestCase, TestCaseCategory } from '../types/foxApi'
import EmptyState from './ui/EmptyState.vue'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import Menu, { type MenuItem } from './ui/Menu.vue'
import TestCaseDrawer from './TestCaseDrawer.vue'
import TestCaseModal from './TestCaseModal.vue'
import { useToast } from '../composables/useToast'

const props = defineProps<{ draft: Endpoint | null }>()

const store = useWorkspaceStore()
const toast = useToast()

type FilterKey = '全部' | TestCaseCategory

const filter = ref<FilterKey>('全部')

/** 当前接口的用例（后端按创建时间排序，克隆/追加在尾部）。 */
const cases = computed<TestCase[]>(() =>
  props.draft ? (store.testCases.get(props.draft.id) ?? []) : [],
)

const filtered = computed<TestCase[]>(() =>
  filter.value === '全部' ? cases.value : cases.value.filter((c) => c.category === filter.value),
)

/** 分类计数（全部 (N) + 各分类）。 */
const counts = computed<Record<FilterKey, number>>(() => {
  const all: Record<FilterKey, number> = {
    全部: cases.value.length,
    正向: 0,
    负向: 0,
    边界值: 0,
    安全性: 0,
    其他: 0,
  }
  for (const c of TEST_CASE_CATEGORIES) {
    all[c] = cases.value.filter((x) => x.category === c).length
  }
  return all
})
// ---------- 新建 / 编辑 ----------
const modalOpen = ref(false)
const editingCase = ref<TestCase | null>(null)

function openCreate(): void {
  editingCase.value = null
  modalOpen.value = true
}

async function onModalSubmit(payload: { name: string; category: TestCaseCategory }): Promise<void> {
  const d = props.draft
  if (!d) return
  if (editingCase.value) {
    const ok = await store.renameTestCase(d.id, editingCase.value.id, payload.name, payload.category)
    if (ok) toast.success('用例已更新')
  } else {
    await store.saveTestCase(d.id, payload.name, payload.category, d.request, d.path, d.method)
  }
}

// ---------- 行操作 ----------
const menuEl = ref<InstanceType<typeof Menu> | null>(null)
const menuTarget = ref<TestCase | null>(null)

function openRowMenu(event: MouseEvent, c: TestCase): void {
  menuTarget.value = c
  const items: MenuItem[] = [
    { key: 'run', label: '直接运行', icon: 'play' },
    { key: 'edit', label: '编辑用例', icon: 'pencil' },
    { key: 'open-debug', label: '在调试页打开', icon: 'layout-grid' },
    { key: 'clone', label: '克隆', icon: 'copy', dividerBefore: true },
    { key: 'delete', label: '删除', icon: 'trash', danger: true, confirm: `删除用例「${c.name}」？删除后不可恢复。` },
  ]
  menuEl.value?.openAt(event.currentTarget as HTMLElement, items, 'left')
}

async function onMenuSelect(item: MenuItem): Promise<void> {
  const c = menuTarget.value
  const d = props.draft
  if (!c || !d) return
  switch (item.key) {
    case 'run':
      await runOne(c)
      break
    case 'edit':
      openDrawer(c)
      break
    case 'open-debug':
      store.openTestCaseInDebug(d.id, c)
      break
    case 'clone':
      await store.cloneTestCase(d.id, c)
      break
  }
}

async function onMenuConfirm(item: MenuItem): Promise<void> {
  const c = menuTarget.value
  const d = props.draft
  if (!c || !d || item.key !== 'delete') return
  await store.removeTestCase(d.id, c.id)
}

// ---------- 详情 / 编辑抽屉 ----------
const drawerOpen = ref(false)
const drawerCase = ref<TestCase | null>(null)

function openDrawer(c: TestCase): void {
  drawerCase.value = c
  drawerOpen.value = true
}

/** 抽屉「立即运行」：原地执行（不切 Tab），结果留在抽屉内。 */
async function runDrawerPayload(payload: {
  method: string
  urlPath: string
  params: TestCase['params']
  headers: TestCase['headers']
  bodyType: string
  bodyContent: string
}): Promise<import('../types/foxApi').ExecuteResponse | null> {
  const d = props.draft
  if (!d || !drawerCase.value) return null
  const snapshot: TestCase = {
    ...drawerCase.value,
    method: payload.method as TestCase['method'],
    url_path: payload.urlPath,
    params: payload.params,
    headers: payload.headers,
    body_type: payload.bodyType,
    body_content: payload.bodyContent,
  }
  return store.runTestCase(d.id, snapshot, store.activeEnvId)
}

/** 抽屉「保存修改」：名称/分组 + 请求内容一起落库。 */
async function saveDrawerPayload(payload: {
  name: string
  category: TestCaseCategory
  method: string
  urlPath: string
  params: TestCase['params']
  headers: TestCase['headers']
  bodyType: string
  bodyContent: string
}): Promise<void> {
  const d = props.draft
  if (!d || !drawerCase.value) return
  const c = drawerCase.value
  await store.updateTestCaseContent(d.id, c.id, {
    method: payload.method as TestCase['method'],
    urlPath: payload.urlPath,
    params: payload.params,
    headers: payload.headers,
    bodyType: payload.bodyType,
    bodyContent: payload.bodyContent,
  })
  const ok = await store.renameTestCase(d.id, c.id, payload.name, payload.category)
  if (ok) toast.success('用例已保存')
}

// ---------- 运行 ----------
const runningIds = ref<Set<string>>(new Set())
const runningAll = ref(false)

async function runOne(c: TestCase): Promise<void> {
  const d = props.draft
  if (!d || runningIds.value.has(c.id)) return
  runningIds.value = new Set(runningIds.value).add(c.id)
  try {
    const res = await store.runTestCase(d.id, c, store.activeEnvId)
    if (res) {
      const ok = res.status >= 200 && res.status < 400
      if (ok) {
        toast.success(`用例通过：${c.name}`, { message: `HTTP ${res.status} · ${res.duration_ms}ms` })
      } else {
        toast.error(`用例失败：${c.name}`, { message: `HTTP ${res.status} · ${res.duration_ms}ms` })
      }
    }
  } catch (err) {
    toast.error(`用例运行失败：${c.name}`, {
      message: err instanceof Error ? err.message : String(err),
    })
  } finally {
    const next = new Set(runningIds.value)
    next.delete(c.id)
    runningIds.value = next
  }
}

async function runAll(): Promise<void> {
  const d = props.draft
  if (!d || runningAll.value) return
  runningAll.value = true
  try {
    const r = await store.runAllTestCases(d.id)
    toast[r.success === r.total ? 'success' : 'info'](
      `全部运行完成：${r.success}/${r.total} 通过`,
    )
  } catch (err) {
    toast.error('全部运行失败', {
      message: err instanceof Error ? err.message : String(err),
    })
  } finally {
    runningAll.value = false
  }
}

const STATUS_TONE: Record<TestCase['last_run_status'], string> = {
  Success: 'var(--ok)',
  Failed: 'var(--danger)',
  Untested: 'var(--text-3)',
}

/** 最近一次运行元信息（状态码 / 耗时），运行结果列富化展示。 */
function runMetaOf(c: TestCase): { status: number; durationMs: number } | undefined {
  return store.caseRunMeta.get(c.id)
}

watch(
  () => props.draft?.id,
  () => {
    filter.value = '全部'
  },
)
</script>

<template>
  <div class="tcp">
    <!-- 顶部：分类筛选 + 操作区 -->
    <div class="tcp-bar">
      <div class="tcp-filters">
        <button
          v-for="key in (['全部', ...TEST_CASE_CATEGORIES] as FilterKey[])"
          :key="key"
          class="tcp-filter"
          :class="{ active: filter === key }"
          type="button"
          @click="filter = key"
        >
          {{ key }}
          <span class="tcp-count">{{ counts[key] }}</span>
        </button>
      </div>
      <div class="tcp-actions">
        <button class="rf-btn rf-btn-sm" type="button" @click="openCreate">
          <Icon name="plus" :size="13" /> 添加用例
        </button>
        <button
          class="rf-btn rf-btn-sm rf-btn-primary"
          type="button"
          :disabled="!cases.length || runningAll"
          @click="runAll"
        >
          <Icon name="play" :size="13" /> {{ runningAll ? '运行中…' : '全部运行' }}
        </button>
      </div>
    </div>

    <!-- 用例表格 -->
    <div v-if="filtered.length" class="tcp-table">
      <div class="tcp-row tcp-head">
        <span class="tcp-col-idx">#</span>
        <span class="tcp-col-name">名称</span>
        <span class="tcp-col-cat">分组</span>
        <span class="tcp-col-status">运行结果</span>
        <span class="tcp-col-ops">操作</span>
      </div>
      <div
        v-for="(c, i) in filtered"
        :key="c.id"
        class="tcp-row tcp-body-row"
      >
        <span class="tcp-col-idx">{{ i + 1 }}</span>
        <button class="tcp-col-name tcp-name-btn" type="button" :title="`查看 / 编辑用例：${c.name}`" @click="openDrawer(c)">
          <span class="tcp-method" :class="`m-select-${c.method.toLowerCase()}`">{{ c.method }}</span>
          <span class="tcp-name-text">{{ c.name }}</span>
        </button>
        <span class="tcp-col-cat">
          <span class="tcp-cat-dot" :style="{ background: CATEGORY_TONE[c.category] }"></span>
          {{ c.category }}
        </span>
        <span class="tcp-col-status">
          <span v-if="runningIds.has(c.id)" class="tcp-status tcp-status-running">
            <span class="tcp-spinner"></span>
            运行中…
          </span>
          <template v-else-if="runMetaOf(c)">
            <span
              class="tcp-status tcp-status-meta"
              :class="`tone-${statusToneOf(runMetaOf(c)!.status)}`"
            >
              <span class="tcp-dot"></span>
              {{ runMetaOf(c)!.status }} {{ statusTextOf(runMetaOf(c)!.status) }}
              <span class="tcp-dur">({{ formatDuration(runMetaOf(c)!.durationMs) }})</span>
            </span>
          </template>
          <span
            v-else
            class="tcp-status"
            :class="`tcp-status-${c.last_run_status.toLowerCase()}`"
            :style="{ color: STATUS_TONE[c.last_run_status] }"
          >
            {{ c.last_run_status === 'Untested' ? '—' : c.last_run_status }}
          </span>
        </span>
        <span class="tcp-col-ops">
          <IconButton
            name="more-horizontal"
            :size="14"
            title="更多操作"
            @click="openRowMenu($event, c)"
          />
        </span>
      </div>
    </div>
    <EmptyState
      v-else
      icon="list"
      compact
      :title="filter === '全部' ? '暂无测试用例' : `「${filter}」分组暂无用例`"
      description="点右上角「+ 添加用例」，或用「保存 ▾ → 保存为用例」把当前请求存为用例"
    />

    <TestCaseModal
      :open="modalOpen"
      :title="editingCase ? '编辑测试用例' : '保存为测试用例'"
      :name="editingCase?.name ?? ''"
      :category="editingCase?.category ?? '正向'"
      @update:open="modalOpen = $event"
      @submit="onModalSubmit"
    />

    <TestCaseDrawer
      :open="drawerOpen"
      :endpoint-id="draft?.id ?? ''"
      :test-case="drawerCase"
      :on-run="runDrawerPayload"
      :on-save="saveDrawerPayload"
      @update:open="drawerOpen = $event"
    />

    <Menu ref="menuEl" @select="onMenuSelect" @confirm="onMenuConfirm" />
  </div>
</template>

<style scoped>
.tcp {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.tcp-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}

.tcp-filters {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-wrap: wrap;
}

.tcp-filter {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  border: 1px solid transparent;
  border-radius: 999px;
  background: none;
  font-family: inherit;
  font-size: 12px;
  color: var(--text-2);
  cursor: pointer;
  transition:
    color var(--dur) var(--ease),
    background var(--dur) var(--ease),
    border-color var(--dur) var(--ease);
}
.tcp-filter:hover {
  color: var(--text-1);
  background: var(--bg-hover);
}
.tcp-filter.active {
  color: var(--accent);
  border-color: var(--accent);
  background: var(--accent-tint, rgba(168, 85, 247, 0.12));
}

.tcp-count {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-3);
}
.tcp-filter.active .tcp-count {
  color: var(--accent);
}

.tcp-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.tcp-table {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}

.tcp-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 10px;
  border: none;
  background: none;
  font-family: inherit;
  text-align: left;
}

.tcp-head {
  height: 30px;
  background: var(--bg-2);
  font-size: 11.5px;
  color: var(--text-3);
}

.tcp-body-row {
  height: 38px;
  border-top: 1px solid var(--border);
  transition: background var(--dur) var(--ease);
}
.tcp-body-row:hover {
  background: var(--bg-hover);
}

.tcp-col-idx {
  width: 34px;
  flex-shrink: 0;
  font-family: var(--font-mono);
  font-size: 11.5px;
  color: var(--text-3);
}

.tcp-col-name {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.tcp-name-btn {
  border: none;
  background: none;
  padding: 0;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  transition: opacity var(--dur) var(--ease);
}
.tcp-name-btn:hover {
  opacity: 0.8;
}

.tcp-method {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  padding: 1px 7px;
  border-radius: 5px;
  border: 1px solid color-mix(in srgb, currentColor 22%, transparent);
  background: color-mix(in srgb, currentColor 12%, transparent);
  font-family: var(--font-mono);
  font-size: 10.5px;
  font-weight: 700;
  letter-spacing: 0.3px;
}

.tcp-name-text {
  font-size: 12.5px;
  color: var(--text-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tcp-col-cat {
  width: 96px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-2);
}

.tcp-cat-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.tcp-col-status {
  width: 148px;
  flex-shrink: 0;
  font-size: 12px;
  font-family: var(--font-mono);
}

.tcp-status-meta {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11.5px;
}
.tcp-status-meta.tone-ok {
  color: var(--ok);
}
.tcp-status-meta.tone-warn {
  color: var(--warning);
}
.tcp-status-meta.tone-err {
  color: var(--danger);
}
.tcp-status-meta.tone-info {
  color: var(--rf-info, #38bdf8);
}

.tcp-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
  box-shadow: 0 0 6px currentColor;
}

.tcp-dur {
  color: var(--text-3);
}

.tcp-col-ops {
  width: 32px;
  flex-shrink: 0;
  display: flex;
  justify-content: flex-end;
}

.tcp-spinner {
  width: 11px;
  height: 11px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: tcp-spin 0.7s linear infinite;
}

@keyframes tcp-spin {
  to {
    transform: rotate(360deg);
  }
}

.tcp-status-running {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}
</style>
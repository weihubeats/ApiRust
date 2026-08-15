<script setup lang="ts">
/**
 * MockRuleDialog：Mock 规则管理（列表 / 新建 / 编辑 / 删除）。
 * 规则在 mock_start 时按 priority 降序匹配，可覆盖接口默认示例。
 */
import { computed, onMounted, ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import type { MockRule } from '../types/foxApi'

const emit = defineEmits<{ close: [] }>()

const store = useWorkspaceStore()
const api = useFoxApi()
const toast = useToast()

const rules = ref<MockRule[]>([])
const busy = ref(false)
const editing = ref<MockRule | null>(null)

const METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']

async function load(): Promise<void> {
  if (!store.project) return
  busy.value = true
  try {
    rules.value = (await api.listMockRules(store.project.id)) ?? []
  } catch (err) {
    toast.error('加载规则失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

onMounted(load)

function blankRule(): MockRule {
  const now = new Date().toISOString()
  return {
    id: crypto.randomUUID(),
    project_id: store.project?.id ?? '',
    endpoint_id: null,
    name: '',
    method: 'GET',
    path: '/',
    match_query: [],
    match_headers: [],
    response_status: 200,
    response_headers: {},
    response_body_template: '',
    delay_ms: 0,
    enabled: true,
    priority: 0,
    created_at: now,
    updated_at: now,
  }
}

function addMatch(row: MockRule, which: 'match_query' | 'match_headers'): void {
  row[which].push({ key: '', value: '' })
}

function removeMatch(row: MockRule, which: 'match_query' | 'match_headers', index: number): void {
  row[which].splice(index, 1)
}

async function save(): Promise<void> {
  if (!editing.value) return
  if (!editing.value.name.trim()) {
    toast.error('请填写规则名称')
    return
  }
  busy.value = true
  try {
    const saved = await api.saveMockRule({ ...editing.value, updated_at: new Date().toISOString() })
    const idx = rules.value.findIndex((r) => r.id === saved.id)
    if (idx === -1) rules.value.push(saved)
    else rules.value[idx] = saved
    editing.value = null
    toast.success('规则已保存')
  } catch (err) {
    toast.error('保存失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

async function remove(rule: MockRule): Promise<void> {
  if (!window.confirm(`删除规则「${rule.name}」？`)) return
  try {
    await api.deleteMockRule(rule.id)
    rules.value = rules.value.filter((r) => r.id !== rule.id)
    if (editing.value?.id === rule.id) editing.value = null
  } catch (err) {
    toast.error('删除失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

const listTitle = computed(() => `Mock 规则 (${rules.value.length})`)
</script>

<template>
  <div class="rule-mask" @click.self="emit('close')">
    <div class="rule-dialog">
      <div class="rule-head">
        <h3 class="rule-title">{{ editing ? '编辑 Mock 规则' : listTitle }}</h3>
        <button class="rf-btn rf-btn-sm" type="button" @click="emit('close')">关闭</button>
      </div>

      <div v-if="editing" class="rule-form">
        <div class="kv-row">
          <input v-model="editing.name" class="rf-input rf-input-sm kv-key" placeholder="规则名称" />
          <select v-model="editing.method" class="rf-input rf-input-sm kv-key">
            <option v-for="m in METHODS" :key="m" :value="m">{{ m }}</option>
          </select>
          <input v-model="editing.path" class="rf-input rf-input-sm kv-value" placeholder="/users/{id}" />
        </div>
        <div class="kv-row">
          <input
            v-model.number="editing.response_status"
            class="rf-input rf-input-sm kv-key"
            type="number"
            min="100"
            max="599"
            placeholder="状态码"
          />
          <input
            v-model.number="editing.delay_ms"
            class="rf-input rf-input-sm kv-key"
            type="number"
            min="0"
            placeholder="延迟 ms"
          />
          <select v-model.number="editing.priority" class="rf-input rf-input-sm kv-key">
            <option :value="0">优先级 0（默认）</option>
            <option :value="1">优先级 1（较高）</option>
            <option :value="2">优先级 2（最高）</option>
          </select>
          <label class="rule-enabled">
            <input v-model="editing.enabled" type="checkbox" /> 启用
          </label>
        </div>
        <div class="rule-sub">Query 匹配</div>
        <div v-for="(m, i) in editing.match_query" :key="i" class="kv-row">
          <input v-model="m.key" class="rf-input rf-input-sm kv-key" placeholder="key" />
          <input v-model="m.value" class="rf-input rf-input-sm kv-value" placeholder="value" />
          <button class="rf-btn rf-btn-sm" type="button" @click="removeMatch(editing, 'match_query', i)">✕</button>
        </div>
        <button class="rf-btn rf-btn-sm" type="button" @click="addMatch(editing, 'match_query')">
          ＋ Query
        </button>
        <div class="rule-sub">Header 匹配</div>
        <div v-for="(m, i) in editing.match_headers" :key="i" class="kv-row">
          <input v-model="m.key" class="rf-input rf-input-sm kv-key" placeholder="key" />
          <input v-model="m.value" class="rf-input rf-input-sm kv-value" placeholder="value" />
          <button class="rf-btn rf-btn-sm" type="button" @click="removeMatch(editing, 'match_headers', i)">✕</button>
        </div>
        <button class="rf-btn rf-btn-sm" type="button" @click="addMatch(editing, 'match_headers')">
          ＋ Header
        </button>
        <textarea
          v-model="editing.response_body_template"
          class="rf-input rule-body"
          spellcheck="false"
          placeholder='响应体模板（支持 {{path.id}} 占位，例如 { "id": "{{id}}" }）'
        ></textarea>
        <div class="kv-row">
          <button class="rf-btn rf-btn-primary rf-btn-sm" type="button" :disabled="busy" @click="save">
            保存
          </button>
          <button class="rf-btn rf-btn-sm" type="button" @click="editing = null">取消</button>
        </div>
      </div>

      <ul v-else-if="rules.length" class="rule-list">
        <li v-for="r in rules" :key="r.id" class="rule-row">
          <span class="rule-method">{{ r.method }}</span>
          <span class="rule-path">{{ r.path }}</span>
          <span class="rule-status">{{ r.response_status }}</span>
          <span class="rule-meta">{{ r.enabled ? '启用' : '停用' }} · 优先级 {{ r.priority }}</span>
          <button class="rf-btn rf-btn-sm" type="button" @click="editing = { ...r }">编辑</button>
          <button class="rf-btn rf-btn-sm" type="button" @click="remove(r)">✕</button>
        </li>
      </ul>
      <p v-else class="rule-hint">暂无规则。Mock 服务默认按接口路径 + 首个响应示例生成行为。</p>

      <button
        v-if="!editing"
        class="rf-btn rf-btn-sm rule-new"
        type="button"
        @click="editing = blankRule()"
      >
        ＋ 新建规则
      </button>
    </div>
  </div>
</template>

<style scoped>
.rule-mask {
  position: fixed;
  inset: 0;
  background: rgba(2, 6, 23, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}

.rule-dialog {
  width: 640px;
  max-width: 92vw;
  max-height: 82vh;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 18px;
  border-radius: 10px;
  border: 1px solid var(--rf-border, #1f2937);
  background: var(--rf-bg-panel-2, #111827);
}

.rule-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.rule-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--rf-text, #f9fafb);
}

.rule-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rule-sub {
  font-size: 12px;
  font-weight: 600;
  color: var(--rf-text-secondary, #9ca3af);
  margin-top: 4px;
}

.rule-body {
  min-height: 90px;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12px;
  resize: vertical;
}

.rule-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.rule-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
}

.rule-method {
  width: 52px;
  flex-shrink: 0;
  font-weight: 700;
  color: var(--rf-text-secondary, #9ca3af);
}

.rule-path {
  flex: 1;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  color: var(--rf-text, #f9fafb);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rule-status {
  width: 40px;
  font-weight: 600;
  color: #34d399;
}

.rule-meta {
  font-size: 11.5px;
  color: var(--rf-text-muted, #6b7280);
}

.rule-hint {
  margin: 0;
  font-size: 12px;
  color: var(--rf-text-muted, #6b7280);
}

.rule-new {
  align-self: flex-start;
}

.rule-enabled {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  color: var(--rf-text, #f9fafb);
}
</style>
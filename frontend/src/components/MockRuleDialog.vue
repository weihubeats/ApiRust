<script setup lang="ts">
/**
 * MockRuleDialog：Mock 规则管理（列表 / 新建 / 编辑 / 删除）。
 * 规则在 mock_start 时按 priority 降序匹配，可覆盖接口默认示例。
 */
import { computed, onMounted, ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import CustomSelect from './ui/CustomSelect.vue'
import CustomNumberInput from './ui/CustomNumberInput.vue'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import Popconfirm from './ui/Popconfirm.vue'
import type { MockRule } from '../types/foxApi'

const emit = defineEmits<{ close: [] }>()

const store = useWorkspaceStore()
const api = useFoxApi()
const toast = useToast()

const rules = ref<MockRule[]>([])
const busy = ref(false)
const editing = ref<MockRule | null>(null)

const METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']
const METHOD_OPTIONS = METHODS.map((m) => ({ value: m, label: m }))
const PRIORITY_OPTIONS = [
  { value: 0, label: '优先级 0（默认）' },
  { value: 1, label: '优先级 1（较高）' },
  { value: 2, label: '优先级 2（最高）' },
]

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
  <Modal :open="true" :title="editing ? '编辑 Mock 规则' : listTitle" width="620px" @close="emit('close')">
    <div v-if="editing" class="rule-form">
      <div class="kv-row">
        <input v-model="editing.name" class="rf-input rf-input-sm kv-key" placeholder="规则名称" />
        <CustomSelect
          :model-value="editing.method"
          :options="METHOD_OPTIONS"
          size="sm"
          @update:model-value="editing.method = String($event) as MockRule['method']"
        >
          <template #display="{ label }">
            <span :class="`rf-method rf-method-${editing.method.toLowerCase()}`">{{ label }}</span>
          </template>
        </CustomSelect>
        <input v-model="editing.path" class="rf-input rf-input-sm kv-value" placeholder="/users/{id}" />
      </div>
      <div class="kv-row">
        <CustomNumberInput
          :model-value="editing.response_status"
          size="sm"
          :min="100"
          :max="599"
          placeholder="状态码"
          @update:model-value="editing.response_status = $event === '' ? 100 : Number($event)"
        />
        <CustomNumberInput
          :model-value="editing.delay_ms"
          size="sm"
          :min="0"
          placeholder="延迟 ms"
          @update:model-value="editing.delay_ms = $event === '' ? 0 : Number($event)"
        />
        <CustomSelect
          :model-value="editing.priority"
          :options="PRIORITY_OPTIONS"
          size="sm"
          @update:model-value="editing.priority = Number($event)"
        />
        <label class="rule-enabled">
          <input v-model="editing.enabled" type="checkbox" /> 启用
        </label>
      </div>
      <div class="rule-sub">Query 匹配</div>
      <div v-for="(m, i) in editing.match_query" :key="i" class="kv-row">
        <input v-model="m.key" class="rf-input rf-input-sm kv-key" placeholder="key" />
        <input v-model="m.value" class="rf-input rf-input-sm kv-value" placeholder="value" />
        <IconButton name="x" :size="13" title="删除" @click="removeMatch(editing, 'match_query', i)" />
      </div>
      <button class="rf-btn rf-btn-sm" type="button" @click="addMatch(editing, 'match_query')">
        <Icon name="plus" :size="13" /> Query
      </button>
      <div class="rule-sub">Header 匹配</div>
      <div v-for="(m, i) in editing.match_headers" :key="i" class="kv-row">
        <input v-model="m.key" class="rf-input rf-input-sm kv-key" placeholder="key" />
        <input v-model="m.value" class="rf-input rf-input-sm kv-value" placeholder="value" />
        <IconButton name="x" :size="13" title="删除" @click="removeMatch(editing, 'match_headers', i)" />
      </div>
      <button class="rf-btn rf-btn-sm" type="button" @click="addMatch(editing, 'match_headers')">
        <Icon name="plus" :size="13" /> Header
      </button>
      <textarea
        v-model="editing.response_body_template"
        class="rf-input rule-body"
        spellcheck="false"
        placeholder='响应体模板（支持 {{path.id}} 占位，例如 { "id": "{{id}}" }）'
      ></textarea>
    </div>

    <ul v-else-if="rules.length" class="rule-list">
      <li v-for="r in rules" :key="r.id" class="rule-row">
        <span class="rule-method">{{ r.method }}</span>
        <span class="rule-path">{{ r.path }}</span>
        <span class="rule-status">{{ r.response_status }}</span>
        <span class="rule-meta">{{ r.enabled ? '启用' : '停用' }} · 优先级 {{ r.priority }}</span>
        <button class="rf-btn rf-btn-sm" type="button" @click="editing = { ...r }">编辑</button>
        <Popconfirm :title="`删除规则「${r.name}」？`" @confirm="remove(r)">
          <IconButton name="trash" :size="13" tone="danger" title="删除" />
        </Popconfirm>
      </li>
    </ul>
    <p v-else class="rule-hint">暂无规则。Mock 服务默认按接口路径 + 首个响应示例生成行为。</p>

    <template #footer>
      <template v-if="editing">
        <button class="rf-btn rf-btn-sm" type="button" @click="editing = null">取消</button>
        <button class="rf-btn rf-btn-primary rf-btn-sm" type="button" :disabled="busy" @click="save">
          保存
        </button>
      </template>
      <template v-else>
        <button
          class="rf-btn rf-btn-sm"
          type="button"
          @click="editing = blankRule()"
        >
          <Icon name="plus" :size="13" /> 新建规则
        </button>
        <button class="rf-btn rf-btn-sm" type="button" @click="emit('close')">关闭</button>
      </template>
    </template>
  </Modal>
</template>

<style scoped>
.rule-form {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rule-sub {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
  margin-top: 4px;
}

.rule-body {
  min-height: 90px;
  font-family: var(--font-mono);
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
  color: var(--text-2);
}

.rule-path {
  flex: 1;
  font-family: var(--font-mono);
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rule-status {
  width: 40px;
  font-weight: 600;
  color: var(--success);
}

.rule-meta {
  font-size: 11.5px;
  color: var(--text-3);
}

.rule-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-3);
}

.rule-enabled {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  color: var(--text-1);
}
</style>
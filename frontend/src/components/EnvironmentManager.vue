<script setup lang="ts">
/**
 * EnvironmentManager：环境管理弹窗（⚙️）。
 * - 左侧：环境列表（色点 + 名称 + 主 baseUrl 标签，激活项高亮）＋「添加环境」/ 删除；
 * - 右侧：名称编辑 + 「基础 URL」专属字段（存为 base_url 变量，相对路径自动拼接）
 *   + 变量表（变量名 / 值 / 启用），值支持行内显示/隐藏（眼睛）；
 * - 保存落库（store.updateEnvironment），取消丢弃本地编辑。
 */
import { computed, ref, watch } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useToast } from '../composables/useToast'
import { envBaseUrl, envColorClass, normalizeBaseUrl } from '../utils/environment'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import Modal from './ui/Modal.vue'
import Popconfirm from './ui/Popconfirm.vue'
import type { Environment } from '../types/foxApi'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ 'update:open': [open: boolean] }>()

const store = useWorkspaceStore()
const toast = useToast()

const envs = ref<Environment[]>([])
const selected = ref<Environment | null>(null)
const busy = ref(false)
const dirty = ref(false)

interface VarRow {
  key: string
  value: string
  enabled: boolean
}

const varRows = ref<VarRow[]>([])
const revealed = ref<Record<number, boolean>>({})

const activeEnvId = computed(() => store.activeEnvId)

/** 基础 URL 专属字段（存储为 base_url 变量，第一等字段，不出现在普通变量表中）。 */
const baseUrl = computed({
  get: () => selected.value?.variables?.base_url ?? '',
  set: (value: string) => {
    if (!selected.value) return
    selected.value.variables = { ...selected.value.variables, base_url: value }
    dirty.value = true
  },
})

function select(env: Environment | null): void {
  selected.value = env ? { ...env } : null
  varRows.value = env
    ? Object.entries(env.variables)
        .filter(([key]) => key !== 'base_url')
        .map(([key, value]) => ({ key, value, enabled: true }))
    : []
  revealed.value = {}
  dirty.value = false
}

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) return
    envs.value = [...store.environments]
    const active = store.environments.find((e) => e.id === store.activeEnvId)
    select(active ?? store.environments[0] ?? null)
  },
)

function addEnvironment(): void {
  if (!store.project) return
  const now = new Date().toISOString()
  const env: Environment = {
    id: crypto.randomUUID(),
    project_id: store.project.id,
    name: '新环境',
    variables: {},
    created_at: now,
    updated_at: now,
  }
  envs.value.push(env)
  select(env)
  dirty.value = true
}

function addVariable(): void {
  varRows.value.push({ key: '', value: '', enabled: true })
  dirty.value = true
}

function removeVariable(index: number): void {
  varRows.value.splice(index, 1)
  dirty.value = true
}

function onAnyChange(): void {
  dirty.value = true
}

async function save(): Promise<void> {
  if (!selected.value || busy.value) return
  const name = selected.value.name.trim()
  if (!name) {
    toast.warning('环境名称不能为空')
    return
  }
  const variables: Record<string, string> = {}
  const normalizedBase = normalizeBaseUrl(baseUrl.value)
  if (normalizedBase) variables.base_url = normalizedBase
  for (const row of varRows.value) {
    if (!row.enabled) continue
    const key = row.key.trim()
    if (!key || key.startsWith('{{') || key.startsWith('$')) continue
    variables[key] = row.value
  }
  busy.value = true
  try {
    const saved = await store.updateEnvironment({ ...selected.value, name, variables })
    selected.value = saved
    envs.value = [...store.environments]
    dirty.value = false
  } catch (err) {
    toast.error('保存环境失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

function cancel(): void {
  envs.value = [...store.environments]
  const active = store.environments.find((e) => e.id === store.activeEnvId)
  select(active ?? store.environments[0] ?? null)
  emit('update:open', false)
}

async function remove(env: Environment): Promise<void> {
  const persisted = store.environments.some((e) => e.id === env.id)
  try {
    if (persisted) await store.deleteEnvironment(env.id)
    const idx = envs.value.findIndex((e) => e.id === env.id)
    envs.value = envs.value.filter((e) => e.id !== env.id)
    if (selected.value?.id === env.id) {
      select(envs.value[Math.min(idx, Math.max(envs.value.length - 1, 0))] ?? null)
    }
  } catch (err) {
    toast.error('删除环境失败', { message: err instanceof Error ? err.message : String(err) })
  }
}
</script>

<template>
  <Modal
    :open="open"
    title="环境管理"
    width="760px"
    @update:open="emit('update:open', $event)"
  >
    <div class="em">
      <aside class="em-list">
        <div class="em-list-head">环境列表</div>
        <div class="em-list-body">
          <div
            v-for="env in envs"
            :key="env.id"
            class="em-row"
            :class="{ active: env.id === activeEnvId, sel: env.id === selected?.id }"
            @click="select(env)"
          >
            <span class="edot" :class="`ed-${envColorClass(env.name)}`"></span>
            <span class="em-row-name" :title="env.name">{{ env.name }}</span>
            <span v-if="envBaseUrl(env)" class="em-row-url">{{ envBaseUrl(env) }}</span>
            <span v-if="env.id === activeEnvId" class="em-row-active">当前</span>
            <Popconfirm
              :title="`删除环境「${env.name}」？删除后不可恢复。`"
              confirm-text="删除"
              @confirm="remove(env)"
            >
              <IconButton
                name="trash"
                :size="12"
                tone="danger"
                title="删除环境"
                class="em-row-del"
              />
            </Popconfirm>
          </div>
          <button class="rf-btn rf-btn-sm em-add" type="button" @click="addEnvironment">
            <Icon name="plus" :size="13" /> 添加环境
          </button>
        </div>
        <p class="em-hint">
          变量名经 <code>&#123;&#123;变量&#125;&#125;</code> 注入请求；未启用的变量不参与注入。
        </p>
      </aside>

      <section class="em-editor">
        <template v-if="selected">
          <div class="em-editor-head">
            <input
              v-model="selected.name"
              class="rf-input em-name"
              placeholder="环境名称（必填）"
              spellcheck="false"
              @input="onAnyChange"
            />
            <span class="em-editor-meta">{{ varRows.filter((r) => r.enabled).length }} 个变量生效</span>
          </div>

          <div class="em-base">
            <label class="em-base-label">基础 URL</label>
            <div class="em-base-field">
              <Icon name="globe" :size="13" class="em-base-icon" />
              <input
                v-model="baseUrl"
                class="rf-input em-base-input"
                placeholder="https://api.example.com"
                spellcheck="false"
                @input="onAnyChange"
              />
            </div>
          </div>
          <p class="em-join-hint">
            相对路径（不以 http 开头）自动拼接基础 URL；请求栏粘贴完整 http(s) 地址时不拼接、直接使用。
          </p>

          <div class="em-table">
            <div class="em-th">
              <span class="em-col-key">变量名</span>
              <span class="em-col-value">值</span>
              <span class="em-col-enabled">启用</span>
              <span class="em-col-op"></span>
            </div>
            <div v-for="(row, i) in varRows" :key="i" class="em-tr" :class="{ off: !row.enabled }">
              <input
                v-model="row.key"
                class="rf-input rf-input-sm em-col-key"
                placeholder="如 base_url"
                spellcheck="false"
                @input="onAnyChange"
              />
              <div class="em-value-wrap">
                <input
                  v-model="row.value"
                  class="rf-input rf-input-sm em-col-value"
                  :type="revealed[i] ? 'text' : 'password'"
                  placeholder="变量值"
                  spellcheck="false"
                  @input="onAnyChange"
                />
                <IconButton
                  name="eye"
                  :size="13"
                  :title="revealed[i] ? '隐藏值' : '显示值'"
                  class="em-reveal"
                  @click="revealed[i] = !revealed[i]"
                />
              </div>
              <input
                v-model="row.enabled"
                type="checkbox"
                class="em-col-enabled"
                :checked="row.enabled"
                @change="onAnyChange"
              />
              <IconButton
                name="trash"
                :size="13"
                tone="danger"
                title="删除变量"
                class="em-col-op"
                @click="removeVariable(i)"
              />
            </div>
            <button class="rf-btn rf-btn-sm em-add-var" type="button" @click="addVariable">
              <Icon name="plus" :size="13" /> 添加变量
            </button>
          </div>
        </template>
        <p v-else class="em-empty">
          暂无环境。点击左侧「添加环境」创建，或从工作区顶部环境选择器进入。
        </p>
      </section>
    </div>

    <template #footer>
      <button class="rf-btn" type="button" @click="cancel">取消</button>
      <button
        class="rf-btn rf-btn-primary"
        type="button"
        :disabled="busy || !dirty"
        @click="save"
      >
        {{ busy ? '保存中…' : '保存' }}
      </button>
    </template>
  </Modal>
</template>

<style scoped>
.em {
  display: flex;
  gap: 14px;
  min-height: 380px;
  max-height: 58vh;
}

/* ---- 左侧：环境列表 ---- */
.em-list {
  width: 232px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  overflow: hidden;
}

.em-list-head {
  padding: 8px 12px;
  font-size: 11px;
  font-weight: 600;
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid var(--border);
}

.em-list-body {
  flex: 1;
  overflow-y: auto;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.em-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius);
  border: 1px solid transparent;
  cursor: pointer;
  transition:
    background var(--dur) var(--ease),
    border-color var(--dur) var(--ease);
}

.em-row:hover {
  background: var(--bg-hover);
}

.em-row.sel {
  background: var(--bg-active);
}

.em-row.active {
  background: var(--accent-tint);
  border-color: rgba(124, 105, 245, 0.4);
}

.em-row-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--text-1);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.em-row-url {
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-3);
  max-width: 74px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-shrink: 1;
}

.em-row-active {
  font-size: 10px;
  line-height: 1.6;
  color: var(--accent);
  background: var(--accent-tint);
  border-radius: 999px;
  padding: 0 6px;
  flex-shrink: 0;
}

.em-row-del {
  margin-left: auto;
  opacity: 0;
  width: 22px;
  height: 22px;
}

.em-row:hover .em-row-del,
.em-row.sel .em-row-del {
  opacity: 1;
}

.em-add {
  margin-top: 4px;
  width: 100%;
  border-style: dashed;
  color: var(--text-2);
}

.em-hint {
  margin: 0;
  padding: 8px 12px;
  border-top: 1px solid var(--border);
  font-size: 10.5px;
  line-height: 1.6;
  color: var(--text-3);
}

.em-hint code {
  font-family: var(--font-mono);
  color: var(--text-2);
}

/* ---- 右侧：变量编辑表 ---- */
.em-editor {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.em-editor-head {
  display: flex;
  align-items: center;
  gap: 10px;
}

.em-name {
  flex: 1;
  height: var(--h-md);
}

.em-editor-meta {
  font-size: 11px;
  color: var(--text-3);
  flex-shrink: 0;
}

.em-base {
  display: flex;
  align-items: center;
  gap: 8px;
}

.em-base-label {
  width: 58px;
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-2);
}

.em-base-field {
  flex: 1;
  min-width: 0;
  position: relative;
  display: flex;
  align-items: center;
}

.em-base-icon {
  position: absolute;
  left: 9px;
  color: var(--text-3);
  pointer-events: none;
}

.em-base-input {
  flex: 1;
  min-width: 0;
  height: var(--h-md);
  padding-left: 28px;
}

.em-join-hint {
  margin: -4px 0 0;
  font-size: 11px;
  line-height: 1.6;
  color: var(--text-3);
}

.em-table {
  flex: 1;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-panel);
  padding: 6px;
}

.em-th {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 2px 8px 6px;
  font-size: 10.5px;
  font-weight: 600;
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.em-tr {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 0;
}

.em-tr.off {
  opacity: 0.45;
}

.em-col-key {
  width: 34%;
  min-width: 0;
}

.em-value-wrap {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 4px;
}

.em-value-wrap .em-col-value {
  flex: 1;
  min-width: 0;
}

.em-reveal {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
}

.em-col-enabled {
  width: 42px;
  flex-shrink: 0;
  accent-color: var(--accent);
  cursor: pointer;
}

.em-col-op {
  width: 24px;
  height: 24px;
  flex-shrink: 0;
}

.em-add-var {
  margin-top: 4px;
  border-style: dashed;
  color: var(--text-2);
}

.em-empty {
  margin: auto;
  font-size: 12.5px;
  color: var(--text-3);
  text-align: center;
}

/* ---- 环境色点 ---- */
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
</style>

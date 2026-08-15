<script setup lang="ts">
/**
 * EnvironmentQuickView：当前环境变量速览弹层（👁️）。
 * - 深色浮层，跟随 EnvironmentBar 定位（空间不足时向上翻转）；
 * - 只读表格 + 值列行内快速编辑（blur / Enter 自动保存）；
 * - 底部：「＋ 添加变量」快捷行 +「管理环境」入口。
 */
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useToast } from '../composables/useToast'
import { envBaseUrl, envColorClass, normalizeBaseUrl } from '../utils/environment'
import Icon from './ui/Icon.vue'

const props = defineProps<{ anchor: HTMLElement | null }>()
const emit = defineEmits<{ close: []; manage: [] }>()

const store = useWorkspaceStore()
const toast = useToast()

const env = computed(
  () => store.environments.find((e) => e.id === store.activeEnvId) ?? null,
)

interface Row {
  key: string
  value: string
}

const rows = ref<Row[]>([])
const baseUrlValue = ref('')
const busy = ref(false)
const dirty = ref(false)
const popupEl = ref<HTMLElement | null>(null)
const pos = ref({ left: 0, top: 0, up: false })

function toRows(environment: typeof env.value): Row[] {
  return Object.entries(environment?.variables ?? {})
    .filter(([key]) => key !== 'base_url')
    .map(([key, value]) => ({ key, value }))
}

watch(
  () => env.value?.id,
  () => {
    rows.value = toRows(env.value)
    baseUrlValue.value = env.value?.variables?.base_url ?? ''
    dirty.value = false
  },
  { immediate: true },
)

function position(): void {
  const el = props.anchor
  if (!el) return
  const rect = el.getBoundingClientRect()
  const width = 400
  const estimate = Math.min(rows.value.length * 32 + 130, 320)
  const spaceBelow = window.innerHeight - rect.bottom - 10
  const up = spaceBelow < estimate && rect.top > estimate
  const left = Math.max(8, Math.min(rect.right - width, window.innerWidth - width - 8))
  pos.value = {
    left,
    top: up ? Math.max(8, rect.top - estimate) : rect.bottom + 8,
    up,
  }
}

function onDocMouseDown(event: MouseEvent): void {
  const target = event.target as Node
  if (popupEl.value?.contains(target) || props.anchor?.contains(target)) return
  emit('close')
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') {
    event.preventDefault()
    emit('close')
  }
}

async function save(): Promise<void> {
  if (!env.value || busy.value) return
  const variables: Record<string, string> = {}
  const normalizedBase = normalizeBaseUrl(baseUrlValue.value)
  if (normalizedBase) variables.base_url = normalizedBase
  for (const row of rows.value) {
    const key = row.key.trim()
    if (!key || key.startsWith('{{') || key.startsWith('$')) continue
    variables[key] = row.value
  }
  busy.value = true
  try {
    const saved = await store.updateEnvironment({ ...env.value, variables }, { silent: true })
    rows.value = toRows(saved)
    baseUrlValue.value = saved.variables?.base_url ?? ''
    dirty.value = false
  } catch (err) {
    toast.error('保存环境失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

function onEdit(): void {
  dirty.value = true
}

function addVariable(): void {
  rows.value.push({ key: '', value: '' })
  dirty.value = true
  void nextTick(() => {
    const inputs = popupEl.value?.querySelectorAll<HTMLInputElement>('.qv-row input.qv-key')
    inputs?.[inputs.length - 1]?.focus()
    position()
  })
}

function onReposition(): void {
  if (popupEl.value) position()
}

onMounted(() => {
  position()
  document.addEventListener('mousedown', onDocMouseDown, true)
  document.addEventListener('keydown', onKeydown)
  window.addEventListener('scroll', onReposition, true)
  window.addEventListener('resize', onReposition)
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onDocMouseDown, true)
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('scroll', onReposition, true)
  window.removeEventListener('resize', onReposition)
  if (dirty.value) void save()
})
</script>

<template>
  <Teleport to="body">
    <div
      v-if="env"
      ref="popupEl"
      class="qv"
      :class="{ up: pos.up }"
      :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
    >
      <div class="qv-head">
        <span class="edot" :class="`ed-${envColorClass(env.name)}`"></span>
        <span class="qv-env-name">{{ env.name }}</span>
        <span class="qv-env-url">{{ envBaseUrl(env) }}</span>
      </div>

      <div class="qv-base">
        <Icon name="globe" :size="13" class="qv-base-icon" />
        <input
          v-model="baseUrlValue"
          class="qv-input qv-base-input"
          spellcheck="false"
          :disabled="busy"
          placeholder="基础 URL，如 https://api.example.com"
          @input="onEdit"
          @blur="save"
          @keydown.enter="(e) => (e.target as HTMLInputElement).blur()"
        />
      </div>

      <div v-if="rows.length" class="qv-table">
        <div class="qv-row qv-row-head">
          <span class="qv-key">变量名</span>
          <span class="qv-value">值</span>
        </div>
        <div v-for="(row, i) in rows" :key="i" class="qv-row">
          <input
            v-model="row.key"
            class="qv-input qv-key"
            spellcheck="false"
            :disabled="busy"
            placeholder="变量名"
            @input="onEdit"
            @blur="save"
            @keydown.enter="(e) => (e.target as HTMLInputElement).blur()"
          />
          <input
            v-model="row.value"
            class="qv-input qv-value"
            spellcheck="false"
            :disabled="busy"
            :placeholder="row.key ? '' : '值'"
            @input="onEdit"
            @blur="save"
            @keydown.enter="(e) => (e.target as HTMLInputElement).blur()"
          />
        </div>
      </div>
      <p v-else class="qv-empty">该环境暂无变量，点击下方「添加变量」。</p>

      <div class="qv-foot">
        <button class="rf-btn rf-btn-sm rf-btn-ghost" type="button" @click="addVariable">
          <Icon name="plus" :size="13" /> 添加变量
        </button>
        <button class="rf-btn rf-btn-sm" type="button" @click="emit('manage')">
          <Icon name="settings" :size="13" /> 管理环境
        </button>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.qv {
  position: fixed;
  z-index: 900;
  width: 400px;
  max-width: calc(100vw - 16px);
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
  animation: qv-in 140ms var(--ease);
  transform-origin: top;
}
.qv.up {
  animation-name: qv-in-up;
  transform-origin: bottom;
}

.qv-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
}

.qv-env-name {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-1);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.qv-env-url {
  font-family: var(--font-mono);
  font-size: 10.5px;
  color: var(--text-3);
  margin-left: auto;
  max-width: 180px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 基础 URL 行（第一等字段） */
.qv-base {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
}

.qv-base-icon {
  color: var(--text-3);
  flex-shrink: 0;
}

.qv-base-input {
  flex: 1;
  min-width: 0;
  height: 28px;
  border: 1px solid transparent;
  background: transparent;
  border-radius: var(--radius);
  padding: 2px 8px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-1);
  outline: none;
  transition:
    border-color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.qv-base-input:hover {
  background: var(--bg-hover);
}
.qv-base-input:focus {
  background: var(--bg-card);
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint);
}

.qv-table {
  max-height: 240px;
  overflow-y: auto;
  padding: 6px 0;
}

.qv-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 3px 12px;
}

.qv-row-head {
  padding: 2px 12px 6px;
  font-size: 10.5px;
  font-weight: 600;
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.qv-key {
  width: 132px;
  flex-shrink: 0;
}

.qv-input.qv-key,
.qv-input.qv-value {
  height: 26px;
  border: 1px solid transparent;
  background: transparent;
  border-radius: var(--radius);
  padding: 2px 8px;
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-1);
  outline: none;
  transition:
    border-color var(--dur) var(--ease),
    background var(--dur) var(--ease);
}
.qv-input.qv-key:hover,
.qv-input.qv-value:hover {
  background: var(--bg-hover);
}
.qv-input.qv-key:focus,
.qv-input.qv-value:focus {
  background: var(--bg-card);
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-tint);
}

.qv-value {
  flex: 1;
  min-width: 0;
}

.qv-empty {
  margin: 0;
  padding: 18px 12px;
  font-size: 12px;
  color: var(--text-3);
  text-align: center;
}

.qv-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 12px;
  border-top: 1px solid var(--border);
  background: var(--bg-panel);
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

@keyframes qv-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
@keyframes qv-in-up {
  from {
    opacity: 0;
    transform: translateY(4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>

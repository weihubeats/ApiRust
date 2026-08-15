<script setup lang="ts">
/**
 * SettingsView：设置页（当前含备份/恢复）。
 * 导出 = 当前激活项目全量 JSON 下载；导入 = 选择备份文件恢复为全新项目。
 */
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import type { Environment, Project } from '../types/foxApi'

const api = useFoxApi()
const toast = useToast()
const router = useRouter()

const project = ref<Project | null>(null)
const busy = ref(false)
const fileInput = ref<HTMLInputElement | null>(null)

// ---------- 环境管理 ----------
const environments = ref<Environment[]>([])
const envError = ref('')
const selectedEnv = ref<Environment | null>(null)
const envDirty = ref(false)

interface VarRow {
  key: string
  value: string
}

function variablesToRows(vars: Record<string, string>): VarRow[] {
  return Object.entries(vars).map(([key, value]) => ({ key, value }))
}

const varRows = ref<VarRow[]>([])

async function loadEnvironments(): Promise<void> {
  if (!project.value) return
  try {
    environments.value = (await api.listEnvironments(project.value.id)) ?? []
  } catch (err) {
    envError.value = err instanceof Error ? err.message : String(err)
  }
}

function selectEnv(env: Environment): void {
  selectedEnv.value = { ...env }
  varRows.value = variablesToRows(env.variables ?? {})
  envDirty.value = false
}

function addVar(): void {
  varRows.value.push({ key: '', value: '' })
  envDirty.value = true
}

function removeVar(index: number): void {
  varRows.value.splice(index, 1)
  envDirty.value = true
}

function onVarChange(): void {
  envDirty.value = true
}

async function saveEnv(): Promise<void> {
  if (!selectedEnv.value) return
  const variables: Record<string, string> = {}
  for (const row of varRows.value) {
    if (!row.key.startsWith('{{') && !row.key.startsWith('$')) {
      variables[row.key] = row.value
    }
  }
  busy.value = true
  try {
    const saved = await api.saveEnvironment({
      ...selectedEnv.value,
      variables,
      updated_at: new Date().toISOString(),
    })
    const idx = environments.value.findIndex((e) => e.id === saved.id)
    if (idx !== -1) environments.value[idx] = saved
    selectedEnv.value = saved
    envDirty.value = false
    toast.success(`已保存环境：${saved.name}`)
  } catch (err) {
    toast.error('保存环境失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

async function newEnv(): Promise<void> {
  if (!project.value) return
  const name = window.prompt('环境名称')
  if (!name?.trim()) return
  const now = new Date().toISOString()
  const env: Environment = {
    id: crypto.randomUUID(),
    project_id: project.value.id,
    name: name.trim(),
    variables: {},
    created_at: now,
    updated_at: now,
  }
  try {
    const saved = await api.saveEnvironment(env)
    environments.value.push(saved)
    selectEnv(saved)
    toast.success(`环境已创建：${saved.name}`)
  } catch (err) {
    toast.error('创建环境失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

onMounted(async () => {
  try {
    project.value = (await api.getActiveProject()) ?? null
  } catch {
    project.value = null
  }
  if (project.value) {
    await loadEnvironments()
  }
})

async function exportBackup(): Promise<void> {
  if (!project.value) return
  busy.value = true
  try {
    const text = await api.backupExport(project.value.id)
    const blob = new Blob([text], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    const stamp = new Date().toISOString().slice(0, 10)
    a.href = url
    a.download = `${project.value.name}-备份-${stamp}.json`
    a.click()
    URL.revokeObjectURL(url)
    toast.success('备份已导出')
  } catch (err) {
    toast.error('导出失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

async function onImportFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  busy.value = true
  try {
    const text = await file.text()
    const summary = await api.backupRestore(text)
    toast.success(
      `已恢复为「${summary.name}」：接口 ${summary.endpoints} 个、环境 ${summary.environments} 个`,
    )
    router.push('/projects')
  } catch (err) {
    toast.error('导入失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="rf-panel settings">
    <h1 class="rf-heading">设置</h1>

    <section class="settings-section">
      <h2 class="rf-subheading">备份与恢复</h2>
      <p class="rf-hint">
        {{ project ? `当前激活项目：${project.name}` : '未选择项目，请先进入任一项目工作区' }}
      </p>
      <div class="settings-actions">
        <button
          class="rf-btn"
          type="button"
          :disabled="!project || busy"
          @click="exportBackup"
        >
          {{ busy ? '处理中…' : '导出当前项目备份' }}
        </button>
        <button class="rf-btn" type="button" :disabled="busy" @click="fileInput?.click()">
          导入备份文件
        </button>
        <input
          ref="fileInput"
          type="file"
          accept=".json,application/json"
          class="settings-file"
          @change="onImportFile"
        />
      </div>
    </section>

    <section class="settings-section">
      <h2 class="rf-subheading">环境管理</h2>
      <p v-if="envError" class="rf-hint env-error">{{ envError }}</p>
      <template v-else-if="project">
        <div class="env-toolbar">
          <select
            class="rf-select env-select"
            :value="selectedEnv?.id ?? ''"
            @change="
              selectEnv(environments.find((e) => e.id === ($event.target as HTMLSelectElement).value)!)
            "
          >
            <option value="" disabled>选择环境…</option>
            <option v-for="env in environments" :key="env.id" :value="env.id">
              {{ env.name }}
            </option>
          </select>
          <button class="rf-btn rf-btn-sm" type="button" @click="newEnv">＋ 新建环境</button>
        </div>
        <div v-if="selectedEnv" class="env-editor">
          <div class="env-name">
            <input v-model="selectedEnv.name" class="rf-input rf-input-sm env-name-input" @input="envDirty = true" />
          </div>
          <div v-for="(row, i) in varRows" :key="i" class="kv-row">
            <input
              v-model="row.key"
              class="rf-input rf-input-sm kv-key"
              placeholder="变量名（如 base_url）"
              @input="onVarChange"
            />
            <input
              v-model="row.value"
              class="rf-input rf-input-sm kv-value"
              placeholder="值"
              spellcheck="false"
              @input="onVarChange"
            />
            <button class="rf-btn rf-btn-sm" type="button" @click="removeVar(i)">✕</button>
          </div>
          <div class="kv-row">
            <button class="rf-btn rf-btn-sm" type="button" @click="addVar">＋ 添加变量</button>
            <button
              class="rf-btn rf-btn-primary rf-btn-sm"
              type="button"
              :disabled="busy || !envDirty"
              @click="saveEnv"
            >
              {{ busy ? '保存中…' : '保存环境（变量名经双花括号注入请求）' }}
            </button>
          </div>
        </div>
        <p v-else class="rf-hint">选择或新建环境后编辑变量。</p>
      </template>
    </section>

    <section class="settings-section">
      <h2 class="rf-subheading">其他</h2>
      <p class="rf-hint">主题、快捷键与数据目录等在后续阶段接入。</p>
    </section>
  </div>
</template>

<style scoped>
.settings {
  gap: 20px;
}

.rf-panel {
  max-width: 640px;
  margin: 0 auto;
  padding: 20px;
  border-radius: 10px;
}

.rf-heading {
  margin: 0 0 16px;
}

.rf-subheading {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}

.rf-hint {
  margin: 0;
}

.settings-section {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 16px;
  border-top: 1px solid var(--rf-border);
}

.settings-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.env-toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
}

.env-select {
  width: 220px;
}

.env-editor {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px;
  border: 1px solid var(--rf-border);
  border-radius: var(--rf-radius);
  background: var(--rf-input-bg);
}

.env-name-input {
  width: 240px;
}

.env-error {
  color: var(--rf-danger);
}

.settings-file {
  display: none;
}
</style>

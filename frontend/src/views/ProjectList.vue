<script setup lang="ts">
/**
 * ProjectList：项目列表视图。
 *
 * 集成统一反馈系统：
 * - useToast：列表加载失败 / 创建成功 / 失败均弹通知；
 * - 加载失败显示重试按钮（重新拉取列表）；
 * - useFieldValidation：项目名称实时校验（必填 + 长度上限）；
 * - 顶部进度条由 useFoxApi 内部自动驱动（ProgressBar 组件全局挂载）。
 *
 * 样式沿用 rf- 设计系统（变量与 fox-desktop/src/styles.rs 对齐）。
 */
import { onMounted, ref } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useFieldValidation } from '../composables/useFieldValidation'
import { useToast } from '../composables/useToast'
import type { Project } from '../types/foxApi'

const api = useFoxApi()
const toast = useToast()

const NAME_MAX = 50

const projects = ref<Project[]>([])
const newName = ref('')
const loading = ref(false)
const loadError = ref<string | null>(null)

const nameField = useFieldValidation(newName, [
  { rule: 'required', message: '项目名称不能为空' },
  { rule: 'maxLength', max: NAME_MAX, message: `项目名称不能超过 ${NAME_MAX} 个字符` },
])

async function load() {
  if (loading.value) return
  loading.value = true
  loadError.value = null
  try {
    projects.value = await api.getProjects()
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e)
    loadError.value = message
    toast.error('项目列表加载失败', { message, duration: 6000 })
  } finally {
    loading.value = false
  }
}

async function createProject() {
  if (!nameField.validate()) {
    toast.warning(nameField.error.value ?? '请先修正输入')
    return
  }
  const name = newName.value.trim()
  const now = new Date().toISOString()
  try {
    const project = await api.saveProject({
      id: crypto.randomUUID(),
      name,
      description: '',
      variables: {},
      created_at: now,
      updated_at: now,
    })
    projects.value.push(project)
    newName.value = ''
    nameField.reset()
    toast.success('项目创建成功', { message: name })
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e)
    toast.error('创建项目失败', { message, duration: 6000 })
  }
}

async function enter(project: Project) {
  try {
    await api.setActiveProject(project.id)
    await api.listEndpoints(project.id)
    toast.info(`已进入项目：${project.name}`)
  } catch (e) {
    toast.error('进入项目失败', {
      message: e instanceof Error ? e.message : String(e),
      duration: 6000,
    })
  }
}

onMounted(load)
</script>

<template>
  <div class="project-list">
    <div class="rf-panel">
      <h1 class="rf-heading">项目列表</h1>

      <form class="rf-form" @submit.prevent="createProject">
        <div class="rf-field">
          <input
            v-model="newName"
            class="rf-input"
            :class="{ 'rf-input-error': nameField.error }"
            placeholder="项目名称"
            maxlength="60"
            @input="nameField.onInput"
            @blur="nameField.onBlur"
            @keyup.enter="createProject"
          />
          <p v-if="nameField.error" class="rf-field-error" role="alert">
            {{ nameField.error }}
          </p>
        </div>
        <button
          class="rf-btn rf-btn-primary"
          type="submit"
          :disabled="api.pending.value || !nameField.valid"
        >
          {{ api.pending.value ? '创建中…' : '新建' }}
        </button>
      </form>

      <div v-if="loadError" class="rf-inline-error" role="alert">
        <span class="rf-inline-error-text">加载失败：{{ loadError }}</span>
        <button class="rf-btn rf-btn-sm" type="button" :disabled="loading" @click="load">
          {{ loading ? '重试中…' : '重试' }}
        </button>
      </div>

      <ul v-if="projects.length" class="rf-list">
        <li v-for="p in projects" :key="p.id" class="rf-list-item">
          <button class="rf-list-btn" type="button" @click="enter(p)">
            {{ p.name }}
            <span v-if="api.activeProject.value?.id === p.id" class="rf-tag">当前</span>
          </button>
          <span class="rf-list-meta">{{ p.created_at.slice(0, 10) }}</span>
        </li>
      </ul>
      <p v-else-if="loading" class="rf-hint">加载中…</p>
      <p v-else-if="!loadError" class="rf-hint">暂无项目，输入名称创建第一个。</p>
    </div>
  </div>
</template>

<style scoped>
.project-list {
  padding: 24px;
}

.rf-panel {
  max-width: 640px;
  margin: 0 auto;
  padding: 20px;
  border-radius: 10px;
  background: var(--rf-bg-panel-2, #111827);
  border: 1px solid var(--rf-border, #1f2937);
}

.rf-heading {
  margin: 0 0 16px;
  font-size: 16px;
  font-weight: 600;
  color: var(--rf-text, #f9fafb);
}

.rf-form {
  display: flex;
  gap: 8px;
  align-items: flex-start;
}

.rf-field {
  flex: 1;
}

.rf-input {
  width: 100%;
  box-sizing: border-box;
  padding: 7px 10px;
  border-radius: 6px;
  border: 1px solid var(--rf-border, #1f2937);
  background: var(--rf-input-bg, #0f172a);
  color: var(--rf-text, #f9fafb);
  font-size: 13px;
  outline: none;
  transition:
    border-color 0.15s ease,
    box-shadow 0.15s ease;
}

.rf-input:focus {
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.25);
}

.rf-input-error {
  border-color: #ef4444;
}

.rf-input-error:focus {
  border-color: #ef4444;
  box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.2);
}

.rf-field-error {
  margin: 5px 0 0;
  font-size: 12px;
  color: #ef4444;
}

.rf-btn {
  border: 1px solid var(--rf-border, #1f2937);
  background: var(--rf-bg-panel, #111827);
  color: var(--rf-text, #f9fafb);
  border-radius: 6px;
  font-size: 13px;
  padding: 7px 14px;
  cursor: pointer;
  white-space: nowrap;
}

.rf-btn-primary {
  background: linear-gradient(135deg, #2563eb, #3b82f6);
  border-color: transparent;
  color: #fff;
}

.rf-btn-primary:disabled {
  opacity: 0.55;
  cursor: default;
}

.rf-btn-sm {
  padding: 4px 10px;
  font-size: 12px;
}

.rf-inline-error {
  margin-top: 14px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 6px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.35);
}

.rf-inline-error-text {
  font-size: 12.5px;
  color: #fca5a5;
  word-break: break-all;
}

.rf-list {
  list-style: none;
  margin: 16px 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.rf-list-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 10px;
  border-radius: 6px;
  border: 1px solid transparent;
  transition: background 0.15s ease;
}

.rf-list-item:hover {
  background: rgba(255, 255, 255, 0.05);
}

.rf-list-btn {
  border: none;
  background: none;
  padding: 0;
  font-size: 13.5px;
  color: var(--rf-text, #f9fafb);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  text-align: left;
}

.rf-tag {
  padding: 1px 7px;
  border-radius: 999px;
  font-size: 11px;
  color: #93c5fd;
  background: rgba(59, 130, 246, 0.15);
  border: 1px solid rgba(59, 130, 246, 0.35);
}

.rf-list-meta {
  font-size: 11.5px;
  color: var(--rf-text-muted, #6b7280);
}

.rf-hint {
  margin: 16px 0 0;
  font-size: 12.5px;
  color: var(--rf-text-secondary, #9ca3af);
}
</style>
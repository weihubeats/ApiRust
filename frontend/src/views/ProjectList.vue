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
import { useRouter } from 'vue-router'
import { useFoxApi } from '../composables/useFoxApi'
import { useFieldValidation } from '../composables/useFieldValidation'
import { useToast } from '../composables/useToast'
import type { Project } from '../types/foxApi'

const api = useFoxApi()
const toast = useToast()
const router = useRouter()

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
    router.push('/workspace')
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e)
    console.error('[ProjectList.enter]', e)
    toast.error('进入项目失败', { message, duration: 6000 })
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
}

.rf-heading {
  margin: 0 0 16px;
}

.rf-field .rf-input {
  width: 100%;
}

.rf-input-error {
  border-color: var(--rf-danger) !important;
}

.rf-input-error:focus {
  border-color: var(--rf-danger) !important;
  box-shadow: 0 0 0 2px rgba(239, 68, 68, 0.2) !important;
}

.rf-list-btn {
  border: none;
  background: none;
  padding: 0;
  font-size: 13.5px;
  color: var(--rf-text);
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  text-align: left;
}

.rf-list-btn:hover .rf-tag {
  border-color: var(--rf-info);
}
</style>
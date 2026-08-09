<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import type { Project } from '../types/foxApi'

const api = useFoxApi()
const projects = ref<Project[]>([])
const newName = ref('')
const loading = ref(false)

async function load() {
  loading.value = true
  try {
    projects.value = await api.getProjects()
  } catch (e) {
    // e 已映射为带 code 的 Error（err.code === 'VALIDATION' | 'DATABASE' | ...）
    console.error(e)
  } finally {
    loading.value = false
  }
}

async function createProject() {
  const name = newName.value.trim()
  if (!name) return
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
    await api.setActiveProject(project.id)
  } catch (e) {
    console.error(e)
  }
}

async function enter(project: Project) {
  await api.setActiveProject(project.id)
  const endpoints = await api.listEndpoints(project.id)
  console.log('当前项目接口：', endpoints)
}

onMounted(load)
</script>

<template>
  <div>
    <h1>项目列表</h1>
    <div>
      <input v-model="newName" placeholder="项目名称" @keyup.enter="createProject" />
      <button :disabled="api.pending.value" @click="createProject">新建</button>
    </div>
    <ul v-if="projects.length">
      <li v-for="p in projects" :key="p.id" @click="enter(p)">
        {{ p.name }}
        <span v-if="api.activeProject.value?.id === p.id">（当前）</span>
      </li>
    </ul>
    <p v-else-if="loading">加载中…</p>
  </div>
</template>
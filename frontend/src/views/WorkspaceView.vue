<script setup lang="ts">
/**
 * WorkspaceView：工作区主视图。
 * 左侧接口树（文件夹 + 接口，含 CRUD），右侧标签页 + 编辑器 + 响应区。
 * 树操作全部走 workspace store（Pinia），点击接口打开草稿标签页。
 */
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useWorkspaceStore } from '../stores/workspace'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import EndpointTree from '../components/EndpointTree.vue'
import TabBar from '../components/TabBar.vue'
import EndpointEditor from '../components/EndpointEditor.vue'
import CurlImportDialog from '../components/CurlImportDialog.vue'
import ImportDialog from '../components/ImportDialog.vue'
import MockRuleDialog from '../components/MockRuleDialog.vue'

const store = useWorkspaceStore()
const router = useRouter()
const api = useFoxApi()
const toast = useToast()

const loading = ref(false)
const showCurlImport = ref(false)
const curlFolderId = ref<string | null>(null)
const showDocImport = ref(false)
const showMockRules = ref(false)

// ---------- Mock 服务 ----------
const mockAddress = ref<string | null>(null)
const mockBusy = ref(false)

async function refreshMockStatus(): Promise<void> {
  try {
    mockAddress.value = (await api.mockStatus()) ?? null
  } catch {
    mockAddress.value = null
  }
}

async function toggleMock(): Promise<void> {
  if (mockBusy.value) return
  mockBusy.value = true
  try {
    if (mockAddress.value) {
      await api.mockStop()
      mockAddress.value = null
      toast.success('Mock 服务已停止')
    } else {
      mockAddress.value = await api.mockStart()
      toast.success(`Mock 服务已启动：${mockAddress.value}`)
    }
  } catch (err) {
    toast.error('Mock 服务操作失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    mockBusy.value = false
  }
}

async function load(): Promise<void> {
  loading.value = true
  try {
    if (!store.project) {
      const p = await store.init()
      if (!p) {
        router.replace('/projects')
        return
      }
    } else {
      await store.refresh()
    }
  } catch {
    // loadError 已在 store 内写入，界面展示重试
  } finally {
    loading.value = false
  }
}

async function exportOpenapi(): Promise<void> {
  if (!store.project) return
  try {
    const text = await api.exportOpenapi(store.project.id)
    const blob = new Blob([text], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `${store.project.name}-openapi.json`
    a.click()
    URL.revokeObjectURL(url)
    toast.success('已导出 OpenAPI 3.0 JSON')
  } catch (err) {
    toast.error('导出失败', { message: err instanceof Error ? err.message : String(err) })
  }
}

function openCurlImport(folderId: string | null): void {
  curlFolderId.value = folderId
  showCurlImport.value = true
}

onMounted(() => {
  load()
  refreshMockStatus()
})
</script>

<template>
  <div class="workspace">
    <aside class="rf-sidebar">
      <div class="sidebar-head">
        <h2 class="rf-heading">{{ store.project?.name ?? '工作区' }}</h2>
        <button class="rf-btn rf-btn-sm" type="button" @click="router.push('/projects')">项目</button>
        <button class="rf-btn rf-btn-sm" type="button" @click="router.push('/graphql')">GraphQL</button>
      </div>
      <div v-if="store.loadError" class="rf-inline-error" role="alert">
        <span class="rf-inline-error-text">加载失败：{{ store.loadError }}</span>
        <button class="rf-btn rf-btn-sm" type="button" :disabled="loading" @click="load">
          {{ loading ? '重试中…' : '重试' }}
        </button>
      </div>
      <div v-else class="tree-wrap">
        <EndpointTree :folder-id="null" @import-curl="openCurlImport" />
      </div>
    </aside>
    <main class="rf-main">
      <div class="mock-bar" v-if="store.project">
        <span class="mock-status" :class="{ on: mockAddress }">
          {{ mockAddress ? `Mock 运行中 · ${mockAddress}` : 'Mock 未运行' }}
        </span>
        <button class="rf-btn rf-btn-sm" type="button" @click="showDocImport = true">
          导入文档
        </button>
        <button class="rf-btn rf-btn-sm" type="button" @click="exportOpenapi">导出 OpenAPI</button>
        <button class="rf-btn rf-btn-sm" type="button" @click="showMockRules = true">Mock 规则</button>
        <button
          class="rf-btn rf-btn-sm"
          type="button"
          :class="{ 'mock-stop': mockAddress }"
          :disabled="mockBusy"
          @click="toggleMock"
        >
          {{ mockBusy ? '处理中…' : mockAddress ? '停止 Mock' : '启动 Mock' }}
        </button>
      </div>
      <TabBar v-if="store.openTabs.length" />
      <EndpointEditor />
    </main>

    <CurlImportDialog
      v-if="showCurlImport"
      :folder-id="curlFolderId"
      @close="showCurlImport = false"
    />
    <ImportDialog v-if="showDocImport" @close="showDocImport = false" />
    <MockRuleDialog v-if="showMockRules" @close="showMockRules = false" />
  </div>
</template>

<style scoped>
.workspace {
  display: flex;
  height: 100%;
  box-sizing: border-box;
}

.rf-sidebar {
  width: 300px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  padding: 12px;
  border-right: 1px solid var(--rf-border, #1f2937);
  background: var(--rf-bg-panel, #111827);
  overflow-y: auto;
}

.sidebar-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}

.rf-heading {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-wrap {
  flex: 1;
}

.rf-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  background: var(--rf-bg-panel-2, #0b1220);
  overflow: hidden;
}

.rf-inline-error {
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

.mock-bar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  padding: 6px 12px;
  border-bottom: 1px solid var(--rf-border, #1f2937);
}

.mock-status {
  font-size: 12px;
  color: var(--rf-text-muted, #6b7280);
}

.mock-status.on {
  color: #34d399;
}

.mock-stop {
  color: #f87171;
  border-color: rgba(248, 113, 113, 0.4);
}
</style>
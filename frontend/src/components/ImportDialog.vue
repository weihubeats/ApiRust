<script setup lang="ts">
/**
 * ImportDialog：OpenAPI/Swagger/Postman 文档导入。
 * 粘贴文本或选择文件 → 后端解析预览 → 确认后经 workspace store 落库。
 */
import { ref } from 'vue'
import { useWorkspaceStore } from '../stores/workspace'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import type { ImportedEndpoint, ImportFormat } from '../types/foxApi'

const emit = defineEmits<{ close: [] }>()

const store = useWorkspaceStore()
const api = useFoxApi()
const toast = useToast()

const text = ref('')
const busy = ref(false)
const result = ref<{ format: ImportFormat; endpoints: ImportedEndpoint[] } | null>(null)
const fileInput = ref<HTMLInputElement | null>(null)

const FORMAT_LABEL: Record<ImportFormat, string> = {
  openapi30: 'OpenAPI 3.0',
  swagger20: 'Swagger 2.0',
  postman21: 'Postman 集合 v2.1',
  unknown: '无法识别',
}

async function pickFile(event: Event): Promise<void> {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = ''
  if (!file) return
  text.value = await file.text()
  parse()
}

async function parse(): Promise<void> {
  if (!text.value.trim()) return
  busy.value = true
  try {
    result.value = await api.importDocument(text.value)
    toast.success(`识别为 ${FORMAT_LABEL[result.value.format]}，共 ${result.value.endpoints.length} 个接口`)
  } catch (err) {
    result.value = null
    toast.error('解析失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}

async function confirm(): Promise<void> {
  if (!result.value) return
  busy.value = true
  try {
    const summary = await store.importEndpoints(result.value.endpoints)
    toast.success(`已导入 ${summary.endpoints} 个接口（含 ${summary.examples} 个示例）`)
    emit('close')
  } catch (err) {
    toast.error('导入失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="import-mask" @click.self="emit('close')">
    <div class="import-dialog">
      <h3 class="import-title">导入文档</h3>
      <p class="import-hint">支持 OpenAPI 3.0 / Swagger 2.0 / Postman Collection v2.1，自动识别格式。</p>
      <div class="import-actions">
        <textarea
          v-model="text"
          class="rf-input import-text"
          spellcheck="false"
          placeholder="粘贴 OpenAPI / Swagger / Postman JSON…"
        ></textarea>
        <div class="import-tools">
          <button class="rf-btn rf-btn-sm" type="button" :disabled="busy" @click="fileInput?.click()">
            选择文件
          </button>
          <button class="rf-btn rf-btn-sm" type="button" :disabled="busy || !text.trim()" @click="parse">
            {{ busy ? '解析中…' : '解析' }}
          </button>
          <input ref="fileInput" type="file" accept=".json,.yaml,.yml,application/json" class="import-file" @change="pickFile" />
        </div>
      </div>

      <div v-if="result" class="import-preview">
        <p class="import-hint">
          {{ FORMAT_LABEL[result.format] }}：{{ result.endpoints.length }} 个接口
          （{{ result.endpoints.filter((e) => e.folder_hint).length }} 个按分组建文件夹）
        </p>
        <ul class="import-list">
          <li v-for="(ep, i) in result.endpoints.slice(0, 12)" :key="i" class="import-row">
            <span class="import-method">{{ ep.method }}</span>
            <span class="import-path">{{ ep.path }}</span>
          </li>
          <li v-if="result.endpoints.length > 12" class="import-hint">… 其余 {{ result.endpoints.length - 12 }} 个略</li>
        </ul>
        <div class="import-tools">
          <button class="rf-btn rf-btn-primary rf-btn-sm" type="button" :disabled="busy" @click="confirm">
            确认导入
          </button>
          <button class="rf-btn rf-btn-sm" type="button" @click="emit('close')">取消</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.import-mask {
  position: fixed;
  inset: 0;
  background: rgba(2, 6, 23, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}

.import-dialog {
  width: 560px;
  max-width: 90vw;
  max-height: 80vh;
  overflow: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 18px;
  border-radius: 10px;
  border: 1px solid var(--rf-border, #1f2937);
  background: var(--rf-bg-panel-2, #111827);
}

.import-title {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: var(--rf-text, #f9fafb);
}

.import-hint {
  margin: 0;
  font-size: 12px;
  color: var(--rf-text-secondary, #9ca3af);
}

.import-text {
  width: 100%;
  min-height: 140px;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12px;
  resize: vertical;
}

.import-actions,
.import-tools {
  display: flex;
  gap: 8px;
  align-items: center;
}

.import-actions {
  flex-direction: column;
  align-items: stretch;
}

.import-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.import-row {
  display: flex;
  gap: 10px;
  font-size: 12.5px;
}

.import-method {
  width: 56px;
  flex-shrink: 0;
  font-weight: 700;
  color: var(--rf-text-secondary, #9ca3af);
}

.import-path {
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  color: var(--rf-text, #f9fafb);
  word-break: break-all;
}

.import-file {
  display: none;
}
</style>
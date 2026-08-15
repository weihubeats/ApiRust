<script setup lang="ts">
/**
 * CurlImportDialog：cURL 命令导入弹窗。
 * 解析（parse_curl_command）→ 预览 → 填写名称 → 落库为接口（目标文件夹）。
 */
import { computed, ref } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useWorkspaceStore } from '../stores/workspace'
import { useToast } from '../composables/useToast'
import type { CurlParsed } from '../types/foxApi'

const props = defineProps<{ folderId: string | null }>()
const emit = defineEmits<{ close: [] }>()

const api = useFoxApi()
const store = useWorkspaceStore()
const toast = useToast()

const command = ref('')
const parsing = ref(false)
const saving = ref(false)
const error = ref<string | null>(null)
const parsed = ref<CurlParsed | null>(null)

const previewName = computed(() => {
  if (parsed.value) {
    const seg = parsed.value.url.split('/').filter(Boolean).pop()
    return seg || `${parsed.value.method} ${parsed.value.url}`
  }
  return ''
})
const name = ref('')

/** Body 预览：raw 仅在部分模式下存在，统一收敛为文本。 */
const bodyPreview = computed(() => {
  const body = parsed.value?.body
  if (!body) return null
  return 'raw' in body ? `${body.mode}: ${body.raw}` : body.mode
})

async function parse(): Promise<void> {
  if (!command.value.trim()) return
  parsing.value = true
  error.value = null
  try {
    parsed.value = await api.parseCurlCommand(command.value)
    name.value = previewName.value
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    parsed.value = null
  } finally {
    parsing.value = false
  }
}

async function save(): Promise<void> {
  if (!parsed.value) return
  if (!name.value.trim()) {
    toast.warning('请填写接口名称')
    return
  }
  saving.value = true
  try {
    await store.createFromCurl(parsed.value, props.folderId, name.value.trim())
    emit('close')
  } catch (err) {
    toast.error('导入失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="modal-mask" @click.self="emit('close')">
    <div class="modal">
      <h2 class="modal-title">导入 cURL 命令</h2>
      <p class="modal-hint">
        支持 -X / -H / -d / --data / -u 等常用参数（解析器见
        <code>fox-core::curl_parser</code>）。
      </p>
      <textarea
        v-model="command"
        class="rf-input curl-input"
        spellcheck="false"
        placeholder="curl -X POST 'https://api.example.com/users' -H 'Content-Type: application/json' -d '{&quot;name&quot;: &quot;alice&quot;}'"
      ></textarea>
      <div class="modal-actions">
        <button class="rf-btn" type="button" :disabled="parsing || !command.trim()" @click="parse">
          {{ parsing ? '解析中…' : '解析' }}
        </button>
        <button class="rf-btn rf-btn-ghost" type="button" @click="emit('close')">取消</button>
      </div>

      <p v-if="error" class="import-error">{{ error }}</p>

      <div v-if="parsed" class="preview">
        <div class="preview-row">
          <span class="preview-method">{{ parsed.method }}</span>
          <span class="preview-url">{{ parsed.url }}</span>
        </div>
        <div class="preview-row">
          <span class="preview-label">请求头</span>
          <span>{{ parsed.headers.length }} 个</span>
        </div>
        <div class="preview-row" v-if="parsed.body">
          <span class="preview-label">Body</span>
          <pre class="preview-body">{{ bodyPreview }}</pre>
        </div>
        <div class="preview-row" v-if="parsed.auth.type !== 'none'">
          <span class="preview-label">认证</span>
          <span>{{ parsed.auth.type }}</span>
        </div>
        <div class="preview-row">
          <input
            v-model="name"
            class="rf-input rf-input-sm name-input"
            placeholder="接口名称"
            @keyup.enter="save"
          />
          <button
            class="rf-btn rf-btn-primary"
            type="button"
            :disabled="saving || !name.trim()"
            @click="save"
          >
            {{ saving ? '保存中…' : '导入' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-mask {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 3000;
}

.modal {
  width: 560px;
  max-width: calc(100vw - 48px);
  max-height: 80vh;
  overflow-y: auto;
  background: var(--rf-bg-panel, #111827);
  border: 1px solid var(--rf-border, #1f2937);
  border-radius: 10px;
  padding: 18px;
  box-shadow: var(--rf-shadow, 0 8px 24px rgba(0, 0, 0, 0.45));
}

.modal-title {
  margin: 0 0 6px;
  font-size: 15px;
  font-weight: 600;
}

.modal-hint {
  margin: 0 0 10px;
  font-size: 12px;
  color: var(--rf-text-secondary, #9ca3af);
}

.curl-input {
  width: 100%;
  min-height: 90px;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12px;
  resize: vertical;
}

.modal-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.rf-btn-ghost {
  border: none;
  background: none;
  color: var(--rf-text-secondary, #9ca3af);
}

.import-error {
  margin: 10px 0 0;
  padding: 8px 10px;
  border-radius: 6px;
  background: rgba(239, 68, 68, 0.1);
  border: 1px solid rgba(239, 68, 68, 0.35);
  color: #fca5a5;
  font-size: 12px;
}

.preview {
  margin-top: 14px;
  padding: 12px;
  border-radius: 8px;
  background: var(--rf-input-bg, #0f172a);
  border: 1px solid var(--rf-border, #1f2937);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.preview-row {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 12.5px;
}

.preview-method {
  font-weight: 700;
  font-size: 11px;
  padding: 2px 7px;
  border-radius: 4px;
  background: rgba(59, 130, 246, 0.15);
  color: #93c5fd;
}

.preview-url {
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  word-break: break-all;
}

.preview-label {
  color: var(--rf-text-muted, #6b7280);
  width: 56px;
  flex-shrink: 0;
}

.preview-body {
  margin: 0;
  font-family: ui-monospace, 'SF Mono', Menlo, monospace;
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
}

.name-input {
  flex: 1;
}
</style>
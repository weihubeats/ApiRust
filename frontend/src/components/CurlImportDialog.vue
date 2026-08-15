<script setup lang="ts">
/**
 * CurlImportDialog：cURL 命令导入弹窗。
 * 解析（parse_curl_command）→ 预览 → 导入为未命名草稿（保存时弹名称确认框）。
 */
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useWorkspaceStore } from '../stores/workspace'
import { useToast } from '../composables/useToast'
import Modal from './ui/Modal.vue'
import type { CurlParsed } from '../types/foxApi'

const props = defineProps<{ folderId: string | null }>()
const emit = defineEmits<{ close: [] }>()

const api = useFoxApi()
const store = useWorkspaceStore()
const toast = useToast()

const command = ref('')
const parsing = ref(false)
const error = ref<string | null>(null)
const parsed = ref<CurlParsed | null>(null)

/** Body 预览：raw 仅在部分模式下存在，统一收敛为文本。 */
const bodyPreview = computed(() => {
  const body = parsed.value?.body
  if (!body) return null
  return 'raw' in body ? `${body.mode}: ${body.raw}` : body.mode
})

/** 归一化 shell 续行（反斜杠换行 → 空格），避免多行粘贴时解析失败。 */
function normalize(cmd: string): string {
  return cmd.replace(/\\[ \t]*\r?\n/g, ' ')
}

async function parse(): Promise<void> {
  const trimmed = command.value.trim()
  if (!trimmed) return
  parsing.value = true
  error.value = null
  try {
    parsed.value = await api.parseCurlCommand(normalize(trimmed))
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err)
    parsed.value = null
  } finally {
    parsing.value = false
  }
}

/** 输入即自动解析（防抖 400ms），无需手动点「解析」。 */
let parseTimer: number | undefined
watch(command, () => {
  window.clearTimeout(parseTimer)
  if (!command.value.trim()) {
    parsed.value = null
    error.value = null
    return
  }
  parseTimer = window.setTimeout(() => void parse(), 400)
})

onBeforeUnmount(() => {
  window.clearTimeout(parseTimer)
})

function importToEditor(): void {
  if (!parsed.value) return
  store.openCurlDraft(parsed.value, props.folderId)
  toast.success('已导入到编辑器，保存时将提示填写接口名称')
  emit('close')
}
</script>

<template>
  <Modal :open="true" title="导入 cURL 命令" width="560px" @close="emit('close')">
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
    </div>

    <template #footer>
      <button class="rf-btn" type="button" @click="emit('close')">取消</button>
      <button
        class="rf-btn rf-btn-primary"
        type="button"
        :disabled="!parsed"
        @click="importToEditor"
      >
        导入到编辑器
      </button>
    </template>
  </Modal>
</template>

<style scoped>
.modal-hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-2);
}

.curl-input {
  width: 100%;
  min-height: 90px;
  margin-top: 10px;
  font-family: var(--font-mono);
  font-size: 12px;
  resize: vertical;
}

.modal-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.import-error {
  margin: 10px 0 0;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--danger-tint);
  border: 1px solid var(--danger-tint);
  color: var(--danger);
  font-size: 12px;
}

.preview {
  margin-top: 14px;
  padding: 12px;
  border-radius: var(--radius);
  background: var(--bg-card);
  border: 1px solid var(--border);
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
  background: var(--info-tint);
  color: var(--info);
}

.preview-url {
  font-family: var(--font-mono);
  word-break: break-all;
}

.preview-label {
  color: var(--text-3);
  width: 56px;
  flex-shrink: 0;
}

.preview-body {
  margin: 0;
  font-family: var(--font-mono);
  font-size: 12px;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
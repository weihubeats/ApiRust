<script setup lang="ts">
/**
 * CodePanel：生成代码面板（请求 Tab 的 Code 标签页）。
 * 从 ToolsDrawer 提取为独立面板：选择语言 → 生成 → 复制，输出为只读代码预览。
 */
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import CustomSelect from './ui/CustomSelect.vue'
import Icon from './ui/Icon.vue'
import type { CodeLang, Endpoint } from '../types/foxApi'

const props = defineProps<{
  draft: Endpoint | null
  url: string
  /** 打开即按当前语言自动生成，切语言时自动重新生成（导出弹窗用）。 */
  autoGenerate?: boolean
}>()

/** 卸载后丢弃异步结果：防止关闭弹窗后仍在写状态 / 弹 Toast。 */
let disposed = false
onBeforeUnmount(() => {
  disposed = true
})

const api = useFoxApi()
const toast = useToast()

const CODE_LANGS: Array<{ value: CodeLang; label: string }> = [
  { value: 'curl', label: 'cURL' },
  { value: 'python', label: 'Python (requests)' },
  { value: 'js', label: 'JavaScript (fetch)' },
  { value: 'go', label: 'Go (net/http)' },
  { value: 'java', label: 'Java (OkHttp)' },
  { value: 'php', label: 'PHP (cURL)' },
]

const codeLang = ref<CodeLang>('curl')
const generatedCode = ref<string | null>(null)
const generating = ref(false)

async function generateCode(): Promise<void> {
  if (!props.draft) return
  generating.value = true
  try {
    const code = await api.codegenRender({
      lang: codeLang.value,
      method: props.draft.method,
      url: props.url,
      headers: props.draft.request.headers,
      body: props.draft.request.body,
      auth: props.draft.request.auth,
    })
    if (disposed) return
    generatedCode.value = code
  } catch (err) {
    if (disposed) return
    toast.error('生成代码失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    if (!disposed) generating.value = false
  }
}

async function copyCode(): Promise<void> {
  if (!generatedCode.value) return
  try {
    await navigator.clipboard.writeText(generatedCode.value)
    toast.success('已复制到剪贴板')
  } catch {
    toast.error('复制失败，请手动选择文本')
  }
}

onMounted(() => {
  if (props.autoGenerate) void generateCode()
})

watch(codeLang, () => {
  if (props.autoGenerate) void generateCode()
})
</script>

<template>
  <div class="panel">
    <div class="cp-row">
      <CustomSelect
        :model-value="codeLang"
        :options="CODE_LANGS"
        size="sm"
        class="cp-lang-select"
        @update:model-value="codeLang = $event as CodeLang"
      />
      <button class="rf-btn rf-btn-sm" type="button" :disabled="generating" @click="generateCode">
        <Icon name="code" :size="13" />
        {{ generating ? '生成中…' : '生成' }}
      </button>
      <button class="rf-btn rf-btn-sm" type="button" :disabled="!generatedCode" @click="copyCode">
        <Icon name="copy" :size="13" /> 复制
      </button>
    </div>
    <pre v-if="generatedCode" class="cp-preview">{{ generatedCode }}</pre>
    <p v-else class="cp-empty">选择语言后点击「生成」，将按当前请求配置生成可运行的代码片段。</p>
  </div>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.cp-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.cp-lang-select {
  width: 200px;
}

.cp-preview {
  margin: 0;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-code);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.55;
  color: var(--text-1);
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 420px;
  overflow-y: auto;
}

.cp-empty {
  margin: 0;
  padding: 14px 16px;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  font-size: 12px;
  color: var(--text-3);
}
</style>

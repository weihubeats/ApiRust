<script setup lang="ts">
/**
 * CodeSnippetsPanel：文档预览右栏的「多语言代码生成」面板。
 *
 * - Tab 切换语言：cURL / JavaScript / Java / Go / Rust；
 * - 打开即生成，切换语言 / 接口 / URL 变化时自动重新生成；
 * - 右上角一键复制（Tauri 原生剪贴板降级链）。
 */
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import Icon from '../ui/Icon.vue'
import Tabs from '../ui/Tabs.vue'
import type { TabItem } from '../ui/Tabs.vue'
import { useFoxApi } from '../../composables/useFoxApi'
import { useToast } from '../../composables/useToast'
import { copyText } from '../../utils/clipboard'
import type { CodeLang, Endpoint } from '../../types/foxApi'

const props = defineProps<{
  draft: Endpoint
  url: string
}>()

/** 卸载后丢弃异步结果：防止切换视图后仍在写状态 / 弹 Toast。 */
let disposed = false
onBeforeUnmount(() => {
  disposed = true
})

const api = useFoxApi()
const toast = useToast()

/** 文档页代码语言选单（对应后端 Lang）。 */
const LANG_TABS: Array<{ value: CodeLang; label: string }> = [
  { value: 'curl', label: 'cURL' },
  { value: 'js', label: 'JavaScript' },
  { value: 'java', label: 'Java' },
  { value: 'go', label: 'Go' },
  { value: 'rust', label: 'Rust' },
]

const langTabs: TabItem[] = LANG_TABS.map((l) => ({ key: l.value, label: l.label }))

const lang = ref<CodeLang>('curl')
const code = ref('')
const generating = ref(false)

async function generate(): Promise<void> {
  generating.value = true
  try {
    const out = await api.codegenRender({
      lang: lang.value,
      method: props.draft.method,
      url: props.url,
      headers: props.draft.request.headers,
      body: props.draft.request.body,
      auth: props.draft.request.auth,
    })
    if (disposed) return
    code.value = out
  } catch (err) {
    if (disposed) return
    code.value = ''
    toast.error('生成代码失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    if (!disposed) generating.value = false
  }
}

async function copyCode(): Promise<void> {
  if (!code.value) return
  const ok = await copyText(code.value)
  if (ok) {
    toast.success('代码已复制到剪贴板')
  } else {
    toast.error('复制失败，请手动选择文本')
  }
}

onMounted(() => {
  void generate()
})

watch(
  () => [lang.value, props.draft.id, props.url, props.draft.request.body] as const,
  () => {
    if (!disposed) void generate()
  },
)
</script>

<template>
  <section class="csp doc-card">
    <header class="csp-head">
      <h4 class="doc-sec-title">代码生成 (Code)</h4>
      <button class="rf-btn rf-btn-sm" type="button" :disabled="!code" @click="copyCode">
        <Icon name="copy" :size="12" /> 复制代码
      </button>
    </header>
    <Tabs v-model="lang" :tabs="langTabs" size="sm" class="csp-tabs" />
    <div class="csp-body">
      <pre v-if="generating" class="csp-hint">生成中…</pre>
      <pre v-else-if="code" class="csp-code">{{ code }}</pre>
      <p v-else class="csp-hint">无法生成代码（请检查请求配置）</p>
    </div>
  </section>
</template>

<style scoped>
.csp {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.csp-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.csp-body {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-code);
  max-height: 320px;
  overflow: auto;
}

.csp-code,
.csp-hint {
  margin: 0;
  padding: 12px;
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-1);
  white-space: pre-wrap;
  word-break: break-all;
}

.csp-hint {
  color: var(--text-3);
}
</style>

<script setup lang="ts">
/**
 * DocsPanel：文档预览（静态渲染当前接口定义）。
 * 展示方法 / 路径 / 描述 / 认证 / 查询参数 / 请求头 / Body / 测试脚本要点，
 * 便于快速核对接口结构（不改动草稿）。
 */
import { computed } from 'vue'
import { bodyContentOf, bodyTypeLabel, bodyTypeOf } from '../utils/testCases'
import type { Endpoint } from '../types/foxApi'

const props = defineProps<{ draft: Endpoint | null }>()

const spec = computed(() => props.draft?.request ?? null)

const enabledParams = computed(() => spec.value?.params.filter((p) => p.enabled) ?? [])
const enabledHeaders = computed(() => spec.value?.headers.filter((h) => h.enabled) ?? [])

const bodyView = computed(() => {
  const s = spec.value
  if (!s) return null
  return {
    label: bodyTypeLabel(bodyTypeOf(s.body)),
    content: bodyContentOf(s.body),
  }
})

const authLabel = computed(() => {
  const a = spec.value?.auth
  if (!a || a.type === 'none') return '无'
  switch (a.type) {
    case 'bearer':
      return `Bearer Token`
    case 'basic':
      return `Basic Auth`
    case 'apikey':
      return `API Key（${a.in}）`
    case 'oauth2':
      return 'OAuth2'
  }
})
</script>

<template>
  <div v-if="draft" class="doc">
    <div class="doc-head">
      <span class="doc-method" :class="`m-select-${draft.method.toLowerCase()}`">{{ draft.method }}</span>
      <code class="doc-path">{{ draft.path }}</code>
      <span v-if="draft.description" class="doc-desc">{{ draft.description }}</span>
    </div>

    <section v-if="spec" class="doc-sec">
      <h4 class="doc-title">认证方式</h4>
      <p class="doc-line">{{ authLabel }}</p>
    </section>

    <section v-if="enabledParams.length" class="doc-sec">
      <h4 class="doc-title">Query 参数（{{ enabledParams.length }}）</h4>
      <table class="doc-table">
        <thead>
          <tr><th>Key</th><th>Value</th><th>描述</th></tr>
        </thead>
        <tbody>
          <tr v-for="(p, i) in enabledParams" :key="i">
            <td class="doc-kv-key">{{ p.key }}</td>
            <td><code>{{ p.value }}</code></td>
            <td class="doc-kv-desc">{{ p.description }}</td>
          </tr>
        </tbody>
      </table>
    </section>

    <section v-if="enabledHeaders.length" class="doc-sec">
      <h4 class="doc-title">请求头（{{ enabledHeaders.length }}）</h4>
      <table class="doc-table">
        <thead>
          <tr><th>Key</th><th>Value</th><th>描述</th></tr>
        </thead>
        <tbody>
          <tr v-for="(h, i) in enabledHeaders" :key="i">
            <td class="doc-kv-key">{{ h.key }}</td>
            <td><code>{{ h.value }}</code></td>
            <td class="doc-kv-desc">{{ h.description }}</td>
          </tr>
        </tbody>
      </table>
    </section>

    <section v-if="bodyView && bodyView.label !== '无 Body'" class="doc-sec">
      <h4 class="doc-title">请求 Body（{{ bodyView.label }}）</h4>
      <pre class="doc-body"><code>{{ bodyView.content || '（空）' }}</code></pre>
    </section>
  </div>
</template>

<style scoped>
.doc {
  display: flex;
  flex-direction: column;
  gap: 16px;
  max-width: 720px;
  padding: 4px 0;
}

.doc-head {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.doc-method {
  flex-shrink: 0;
  font-family: var(--font-mono);
  font-size: 12.5px;
  font-weight: 700;
}

.doc-path {
  font-family: var(--font-mono);
  font-size: 13px;
  color: var(--text-1);
  background: var(--bg-2);
  padding: 4px 10px;
  border-radius: 6px;
}

.doc-desc {
  font-size: 12.5px;
  color: var(--text-2);
}

.doc-sec {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.doc-title {
  margin: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-3);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}

.doc-line {
  margin: 0;
  font-size: 13px;
  color: var(--text-1);
}

.doc-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12.5px;
}
.doc-table th {
  padding: 5px 10px;
  text-align: left;
  font-size: 11.5px;
  color: var(--text-3);
  border-bottom: 1px solid var(--border);
}
.doc-table td {
  padding: 5px 10px;
  border-bottom: 1px solid var(--border);
  color: var(--text-2);
}
.doc-table code {
  font-family: var(--font-mono);
  font-size: 12px;
  color: var(--text-1);
}

.doc-kv-key {
  font-family: var(--font-mono);
  color: var(--text-1);
}

.doc-kv-desc {
  color: var(--text-3);
}

.doc-body {
  margin: 0;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-2);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-1);
  white-space: pre-wrap;
  word-break: break-all;
  max-height: 320px;
  overflow: auto;
}
</style>
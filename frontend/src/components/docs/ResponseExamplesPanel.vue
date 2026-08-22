<script setup lang="ts">
/**
 * ResponseExamplesPanel：文档预览右栏的「响应示例」面板。
 *
 * - 按状态码分组 Tab（2xx 成功在前，4xx/5xx 错误在后）；
 * - 同状态码多示例时提供示例切换列表；
 * - Body 用 CodeMirror 只读模式渲染（JSON 语法高亮 + 行号 + 折叠）；
 * - 右上角一键复制当前示例 Body。
 */
import { computed, ref, watch } from 'vue'
import JsonCodeMirror from '../JsonCodeMirror.vue'
import EmptyState from '../ui/EmptyState.vue'
import Icon from '../ui/Icon.vue'
import Tabs from '../ui/Tabs.vue'
import type { TabItem } from '../ui/Tabs.vue'
import { useToast } from '../../composables/useToast'
import { copyText } from '../../utils/clipboard'
import { prettyJson } from '../../utils/jsonFormat'
import { statusTextOf } from '../../utils/testCases'
import type { ResponseExample } from '../../types/foxApi'

const props = defineProps<{ examples: ResponseExample[] }>()

const toast = useToast()

/** 状态码分组：2xx 在前、3xx 次之、4xx/5xx 在后，各自升序。 */
const statusTabs = computed<TabItem[]>(() => {
  const statuses = Array.from(new Set(props.examples.map((e) => e.status))).sort((a, b) => {
    const rank = (s: number): number => (s < 400 ? 0 : 1)
    return rank(a) - rank(b) || a - b
  })
  return statuses.map((s) => ({
    key: String(s),
    label: `${s} ${statusTextOf(s)}`,
  }))
})

const activeStatus = ref('')

watch(
  statusTabs,
  (tabs) => {
    if (!tabs.some((t) => t.key === activeStatus.value)) {
      activeStatus.value = tabs[0]?.key ?? ''
    }
  },
  { immediate: true },
)

/** 当前状态码下的示例列表。 */
const statusExamples = computed(() =>
  props.examples.filter((e) => String(e.status) === activeStatus.value),
)

const activeExample = ref<ResponseExample | null>(null)

watch(
  statusExamples,
  (list) => {
    if (!list.some((e) => e.id === activeExample.value?.id)) {
      activeExample.value = list[0] ?? null
    }
  },
  { immediate: true },
)

/** 展示用 Body：JSON 美化（无损），解析失败回退原文。 */
const displayBody = computed(() => {
  const body = activeExample.value?.body ?? ''
  if (!body.trim()) return ''
  try {
    return prettyJson(body)
  } catch {
    return body
  }
})

async function copyBody(): Promise<void> {
  const body = activeExample.value?.body ?? ''
  if (!body) return
  const ok = await copyText(body)
  if (ok) {
    toast.success('已复制响应示例')
  } else {
    toast.error('复制失败，请手动选择文本')
  }
}
</script>

<template>
  <section class="rep doc-card">
    <header class="rep-head">
      <h4 class="doc-sec-title">响应示例 (Response)</h4>
      <span v-if="examples.length" class="rep-count">{{ examples.length }} 条</span>
      <button
        v-if="activeExample"
        class="rf-btn rf-btn-sm"
        type="button"
        @click="copyBody"
      >
        <Icon name="copy" :size="12" /> 复制
      </button>
    </header>

    <template v-if="examples.length">
      <Tabs v-model="activeStatus" :tabs="statusTabs" size="sm" class="rep-tabs" />
      <div v-if="statusExamples.length > 1" class="rep-picker">
        <button
          v-for="ex in statusExamples"
          :key="ex.id"
          type="button"
          class="rep-picker-item"
          :class="{ active: activeExample?.id === ex.id }"
          @click="activeExample = ex"
        >
          {{ ex.name || '未命名示例' }}
        </button>
      </div>
      <div class="rep-meta" v-if="activeExample">
        <span class="rep-status" :class="{ err: activeExample.status >= 400 }">
          {{ activeExample.status }} {{ statusTextOf(activeExample.status) }}
        </span>
        <span v-if="activeExample.content_type" class="rep-ctype">{{ activeExample.content_type }}</span>
        <span class="rep-time">{{ activeExample.updated_at.slice(0, 16).replace('T', ' ') }}</span>
      </div>
      <div class="rep-body">
        <JsonCodeMirror v-if="displayBody" :model-value="displayBody" readonly />
        <p v-else class="rep-empty-body">（该示例无响应 Body）</p>
      </div>
    </template>
    <EmptyState
      v-else
      icon="file"
      title="暂无响应示例"
      description="在调试页发送请求后，可将真实响应保存为示例（200 成功 / 400 错误）"
      compact
    />
  </section>
</template>

<style scoped>
.rep {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.rep-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.rep-count {
  font-size: 11.5px;
  color: var(--text-3);
}

.rep-head .rf-btn {
  margin-left: auto;
}

.rep-tabs :deep(.tabs) {
  border-bottom-color: var(--border);
}

.rep-picker {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.rep-picker-item {
  padding: 2px 10px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg-card);
  color: var(--text-2);
  font-size: 11.5px;
  cursor: pointer;
  transition:
    color var(--dur) var(--ease),
    border-color var(--dur) var(--ease);
}
.rep-picker-item:hover {
  color: var(--text-1);
  border-color: var(--border-strong);
}
.rep-picker-item.active {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  background: var(--accent-tint);
}

.rep-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 11.5px;
  color: var(--text-3);
}

.rep-status {
  font-family: var(--font-mono);
  font-weight: 600;
  color: var(--success);
}
.rep-status.err {
  color: var(--danger);
}

.rep-ctype {
  font-family: var(--font-mono);
}

.rep-body {
  height: 260px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--bg-code);
  overflow: hidden;
}

.rep-empty-body {
  margin: 0;
  padding: 14px;
  font-size: 12px;
  color: var(--text-3);
}
</style>

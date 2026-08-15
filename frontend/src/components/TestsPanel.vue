<script setup lang="ts">
/**
 * TestsPanel：测试（断言）面板——请求 Tab 的 Tests 标签页。
 * 从 ToolsDrawer 提取为独立面板：JSON 断言配置 → 运行 → 逐条结果。
 */
import { ref, watch } from 'vue'
import { useFoxApi } from '../composables/useFoxApi'
import { useToast } from '../composables/useToast'
import { useWorkspaceStore } from '../stores/workspace'
import { formatDuration } from '../utils/format'
import Icon from './ui/Icon.vue'
import type { Endpoint, EndpointResult } from '../types/foxApi'

const props = defineProps<{
  draft: Endpoint | null
  url: string
}>()

const api = useFoxApi()
const toast = useToast()
const store = useWorkspaceStore()

const testsJson = ref('')
const testResult = ref<EndpointResult | null>(null)
const testing = ref(false)

watch(
  () => props.draft,
  () => {
    const tests = (props.draft?.request as { tests?: unknown } | undefined)?.tests
    testsJson.value = tests ? JSON.stringify(tests, null, 2) : ''
  },
  { deep: true, immediate: true },
)

async function runTests(): Promise<void> {
  if (!props.draft) return
  try {
    ;(props.draft.request as { tests?: unknown }).tests = testsJson.value.trim()
      ? JSON.parse(testsJson.value)
      : null
  } catch {
    toast.error('测试配置不是合法 JSON')
    return
  }
  testing.value = true
  try {
    testResult.value = await api.testEndpoint({
      endpoint: props.draft,
      url: props.url,
      environment_id: store.activeEnvId,
    })
  } catch (err) {
    toast.error('测试运行失败', { message: err instanceof Error ? err.message : String(err) })
  } finally {
    testing.value = false
  }
}
</script>

<template>
  <div class="panel">
    <div class="tp-head">
      <span class="tp-title">断言配置 (JSON)</span>
      <span class="tp-hint">保存在当前接口的 request.tests 中，随接口一起保存</span>
    </div>
    <textarea
      v-model="testsJson"
      class="rf-input tp-input"
      spellcheck="false"
      placeholder='{ "assertions": [{ "type": "status", "op": "eq", "expected": 200 }] }'
    ></textarea>
    <div class="tp-run-row">
      <button class="rf-btn rf-btn-sm" type="button" :disabled="testing" @click="runTests">
        <Icon :name="testing ? 'refresh' : 'beaker'" :size="13" :stroke-width="testing ? 1.8 : 1.5" />
        {{ testing ? '测试中…' : '运行测试' }}
      </button>
      <span v-if="testResult" class="tp-badge" :class="testResult.ok ? 'ok' : 'fail'">
        {{ testResult.ok ? '通过' : '失败' }} · {{ testResult.status ?? '-' }} ·
        {{ formatDuration(testResult.duration_ms) }}
      </span>
    </div>
    <ul v-if="testResult?.outcomes.length" class="tp-list">
      <li
        v-for="(o, i) in testResult.outcomes"
        :key="i"
        class="tp-outcome"
        :class="o.passed ? 'ok' : 'fail'"
      >
        <span class="tp-icon">
          <Icon :name="o.passed ? 'check' : 'x'" :size="13" :stroke-width="2" />
        </span>
        <span class="tp-text">{{ o.description }}</span>
        <span v-if="o.reason" class="tp-reason">{{ o.reason }}</span>
      </li>
    </ul>
    <p v-else-if="testResult && !testResult.ok" class="tp-hint-empty">
      {{ testResult.request_error ?? '未配置断言' }}
    </p>
  </div>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.tp-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
}

.tp-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
}

.tp-hint {
  font-size: 11.5px;
  color: var(--text-3);
}

.tp-input {
  width: 100%;
  min-height: 110px;
  font-size: 12px;
  resize: vertical;
}

.tp-run-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.tp-badge {
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 600;
  font-family: var(--font-mono);
}
.tp-badge.ok {
  color: var(--success);
  background: var(--success-tint);
}
.tp-badge.fail {
  color: var(--danger);
  background: var(--danger-tint);
}

.tp-list {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tp-outcome {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 6px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-size: 12px;
}
.tp-outcome.ok {
  border-color: var(--success-tint);
  background: var(--success-tint);
  color: var(--text-1);
}
.tp-outcome.fail {
  border-color: var(--danger-border);
  background: var(--danger-tint);
  color: var(--text-1);
}

.tp-icon {
  flex-shrink: 0;
  display: inline-flex;
}
.tp-outcome.ok .tp-icon {
  color: var(--success);
}
.tp-outcome.fail .tp-icon {
  color: var(--danger);
}

.tp-text {
  font-weight: 500;
}

.tp-reason {
  color: var(--text-2);
  word-break: break-all;
}

.tp-hint-empty {
  margin: 0;
  font-size: 12px;
  color: var(--text-2);
}
</style>

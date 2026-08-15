<script setup lang="ts">
/**
 * ScriptsPanel：脚本面板（前置/后置脚本）。
 * 当前版本暂未提供脚本执行能力，仅展示占位说明与示例。
 */
import type { Endpoint } from '../types/foxApi'

defineProps<{ draft: Endpoint | null }>()
</script>

<template>
  <div class="panel">
    <div class="scripts-empty">
      <p class="scripts-empty-title">脚本（前置 / 后置）</p>
      <p class="scripts-empty-hint">
        支持在请求发送前 / 响应返回后运行 JavaScript 脚本（编辑变量、断言断言结果等），将在后续版本接入。
      </p>
      <pre class="scripts-sample">pre-request:
  pm.setVariable('token', auth.token)

post-response:
  pm.test('status is 200', () =&gt; pm.response.code === 200)</pre>
    </div>
  </div>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.scripts-empty {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 14px 16px;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius);
  background: var(--bg-card);
}

.scripts-empty-title {
  margin: 0;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-1);
}

.scripts-empty-hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-2);
}

.scripts-sample {
  margin: 0;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-panel);
  color: var(--text-2);
  font-family: var(--font-mono);
  font-size: 11.5px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
}
</style>
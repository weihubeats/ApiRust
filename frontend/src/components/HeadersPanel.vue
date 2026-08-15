<script setup lang="ts">
/**
 * HeadersPanel：请求头面板（Postman 式 kv 表格：幽灵行自动追加、空行自动清理）。
 */
import { computed } from 'vue'
import KeyValueTable, { type KVRow } from './ui/KeyValueTable.vue'
import type { Endpoint, KeyValue } from '../types/foxApi'

const props = defineProps<{ draft: Endpoint | null }>()

const headers = computed(() => props.draft?.request.headers ?? [])

function applyHeaders(rows: KVRow[]): void {
  headers.value.splice(0, headers.value.length, ...(rows as KeyValue[]))
}
</script>

<template>
  <div class="panel">
    <KeyValueTable
      :model-value="headers"
      key-placeholder="Header"
      value-placeholder="Value"
      description-placeholder="描述"
      @update:model-value="applyHeaders"
    />
  </div>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
</style>
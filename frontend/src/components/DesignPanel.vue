<script setup lang="ts">
/**
 * DesignPanel：接口设计信息（名称 / 路径 / 描述 / 状态）。
 * 名称与路径与请求栏/面包屑共享同一草稿，这里提供完整字段编辑。
 */
import { computed } from 'vue'
import type { Endpoint, EndpointStatus } from '../types/foxApi'
import CustomSelect from './ui/CustomSelect.vue'

const props = defineProps<{ draft: Endpoint | null }>()

/** 草稿别名：draft 来自 store（可变对象），避免直接在 prop 上写。 */
const d = computed(() => props.draft)

const STATUS_OPTIONS: { value: EndpointStatus; label: string }[] = [
  { value: 'designing', label: '设计中' },
  { value: 'developing', label: '开发中' },
  { value: 'testing', label: '测试中' },
  { value: 'released', label: '已发布' },
  { value: 'deprecated', label: '已废弃' },
]

const statusLabel = computed(() => STATUS_OPTIONS.find((s) => s.value === props.draft?.status)?.label ?? '')

function onStatusChange(v: string | number): void {
  const target = d.value
  if (target) target.status = String(v) as EndpointStatus
}
</script>

<template>
  <div v-if="d" class="dpn">
    <label class="dpn-field">
      <span class="dpn-label">接口名称</span>
      <input v-model="d.name" class="dpn-input" type="text" spellcheck="false" />
    </label>
    <label class="dpn-field">
      <span class="dpn-label">请求路径</span>
      <div class="dpn-path">
        <span class="dpn-method" :class="`m-select-${d.method.toLowerCase()}`">{{ d.method }}</span>
        <input v-model="d.path" class="dpn-input dpn-path-input" type="text" spellcheck="false" />
      </div>
    </label>
    <label class="dpn-field">
      <span class="dpn-label">接口描述</span>
      <textarea
        v-model="d.description"
        class="dpn-input dpn-textarea"
        rows="3"
        placeholder="接口用途、注意事项、返回约定…"
        spellcheck="false"
      ></textarea>
    </label>
    <label class="dpn-field">
      <span class="dpn-label">生命周期状态</span>
      <div class="dpn-status-row">
        <CustomSelect
          :model-value="d.status"
          :options="STATUS_OPTIONS"
          @update:model-value="onStatusChange"
        />
        <span class="dpn-status-hint">当前：{{ statusLabel }}</span>
      </div>
    </label>
  </div>
</template>

<style scoped>
.dpn {
  display: flex;
  flex-direction: column;
  gap: 14px;
  max-width: 560px;
  padding: 4px 0;
}

.dpn-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.dpn-label {
  font-size: 12px;
  color: var(--text-2);
}

.dpn-input {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  font-family: inherit;
  font-size: 13px;
  color: var(--text-1);
  background: var(--bg-1);
  outline: none;
  transition: border-color var(--dur) var(--ease);
}
.dpn-input:focus {
  border-color: var(--accent);
}

.dpn-path {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dpn-method {
  flex-shrink: 0;
  font-family: var(--font-mono);
  font-size: 11.5px;
  font-weight: 700;
}

.dpn-path-input {
  flex: 1;
  font-family: var(--font-mono);
}

.dpn-textarea {
  resize: vertical;
  line-height: 1.5;
}

.dpn-status-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.dpn-status-hint {
  font-size: 12px;
  color: var(--text-3);
}
</style>
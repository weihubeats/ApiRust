<script setup lang="ts">
/**
 * BodyPanel：请求体面板。
 * Tab Bar（Postman 风格）：none / form-data / x-www-form-urlencoded / raw / binary / graphql。
 * - raw 为 json+text 聚合视图：右侧子类型下拉（JSON/Text/JS/HTML/XML）切换
 *   编辑器并联动 Content-Type 请求头（映射逻辑见 utils/bodyMode.ts）；
 * - binary 为本地文件路径，发送时后端读取原始字节作为请求体；
 * - urlencoded/multipart 为字段行编辑，graphql 为 query/variables 编辑器。
 * bodyAny 用 any 放宽联合类型访问（模板 v-model 直写 raw / spec.*）。
 */
import { computed } from 'vue'
import CustomSelect from './ui/CustomSelect.vue'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import JsonEditor from './ui/JsonEditor.vue'
import SegmentedControl, { type SegmentOption } from './ui/SegmentedControl.vue'
import { RAW_SUBTYPES, applyBodyTab, applyRawSubtype, rawSubtypeOf, tabOf } from '../utils/bodyMode'
import type { BodyTab, RawSubtype } from '../utils/bodyMode'
import type { Endpoint, KeyValue, MultipartField } from '../types/foxApi'

const props = defineProps<{ draft: Endpoint | null }>()

const bodyAny = computed(() => props.draft?.request.body as any)
const headersAny = computed(() => props.draft?.request.headers as KeyValue[] | undefined)
const graphql = computed(() => bodyAny.value?.spec as any)

const BODY_TABS: SegmentOption[] = [
  { value: 'none', label: '无' },
  { value: 'form-data', label: 'form-data' },
  { value: 'x-www-form-urlencoded', label: 'x-www-form-urlencoded' },
  { value: 'raw', label: 'raw' },
  { value: 'binary', label: 'binary' },
  { value: 'graphql', label: 'GraphQL' },
]

const RAW_SUBTYPE_OPTIONS = RAW_SUBTYPES.map((s) => ({ value: s.value, label: s.label }))

const activeTab = computed({
  get: () => tabOf(bodyAny.value ?? { mode: 'none' }, headersAny.value ?? []),
  set: (tab: string) => {
    if (props.draft) applyBodyTab(props.draft.request, tab as BodyTab)
  },
})

const rawSubtype = computed({
  get: () => rawSubtypeOf(bodyAny.value ?? { mode: 'none' }, headersAny.value ?? []),
  set: (subtype: string) => {
    if (props.draft) applyRawSubtype(props.draft.request, subtype as RawSubtype)
  },
})

const RAW_PLACEHOLDER: Record<RawSubtype, string> = {
  json: '{ "key": "value" }',
  text: '纯文本内容',
  javascript: '// JavaScript 代码',
  html: '<!DOCTYPE html>\n<html>…</html>',
  xml: '<?xml version="1.0" encoding="UTF-8"?>\n<root>…</root>',
}

const MULTIPART_TYPE_OPTIONS = [
  { value: 'text', label: '文本' },
  { value: 'file_path', label: '文件路径' },
]

function addUrlencodedField(): void {
  const fields = props.draft?.request.body as { fields: KeyValue[] } | undefined
  fields?.fields.push({ key: '', value: '', enabled: true, description: '' })
}

function removeUrlencodedField(index: number): void {
  const fields = props.draft?.request.body as { fields: unknown[] } | undefined
  fields?.fields.splice(index, 1)
}

function addMultipartField(): void {
  const fields = props.draft?.request.body as { fields: MultipartField[] } | undefined
  fields?.fields.push({ key: '', value_type: 'text', value: '', enabled: true })
}

function removeMultipartField(index: number): void {
  const fields = props.draft?.request.body as { fields: unknown[] } | undefined
  fields?.fields.splice(index, 1)
}
</script>

<template>
  <div class="panel">
    <div class="mode-bar">
      <SegmentedControl v-model="activeTab" :options="BODY_TABS" size="sm" class="mode-tabs" />
      <CustomSelect
        v-if="activeTab === 'raw'"
        v-model="rawSubtype"
        :options="RAW_SUBTYPE_OPTIONS"
        size="sm"
        class="raw-subtype"
        pop-class="raw-subtype-pop"
      />
    </div>

    <JsonEditor
      v-if="activeTab === 'raw' && rawSubtype === 'json'"
      v-model="bodyAny.raw"
      placeholder='{ "key": "value" }'
      :min-height="120"
    />
    <textarea
      v-else-if="activeTab === 'raw'"
      v-model="bodyAny.raw"
      class="rf-input body-input"
      spellcheck="false"
      :placeholder="RAW_PLACEHOLDER[rawSubtype as RawSubtype] ?? '纯文本内容'"
    ></textarea>

    <div v-else-if="activeTab === 'graphql'" class="gql-editor">
      <textarea
        v-model="graphql.query"
        class="rf-input body-input"
        spellcheck="false"
        placeholder="query Hero($id: ID!) { hero(id: $id) { name } }"
      ></textarea>
      <JsonEditor
        v-model="graphql.variables"
        placeholder='{ "id": "42" }'
        :min-height="80"
      />
      <input
        v-model="graphql.operation_name"
        class="rf-input rf-input-sm"
        placeholder="operationName（可选）"
      />
    </div>

    <div v-else-if="activeTab === 'x-www-form-urlencoded'" class="editor-fields">
      <div v-for="(f, i) in bodyAny.fields" :key="i" class="kv-row">
        <input v-model="f.enabled" type="checkbox" class="kv-check" />
        <input v-model="f.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
        <input v-model="f.value" class="rf-input rf-input-sm kv-value" placeholder="Value" />
        <IconButton name="x" :size="13" title="删除" @click="removeUrlencodedField(i)" />
      </div>
      <button class="rf-btn rf-btn-sm" type="button" @click="addUrlencodedField">
        <Icon name="plus" :size="13" /> 添加字段
      </button>
    </div>

    <div v-else-if="activeTab === 'form-data'" class="editor-fields">
      <div v-for="(f, i) in bodyAny.fields" :key="i" class="kv-row">
        <input v-model="f.enabled" type="checkbox" class="kv-check" />
        <input v-model="f.key" class="rf-input rf-input-sm kv-key" placeholder="Key" />
        <CustomSelect
          v-model="f.value_type"
          :options="MULTIPART_TYPE_OPTIONS"
          size="sm"
          class="mp-type"
        />
        <input
          v-model="f.value"
          class="rf-input rf-input-sm kv-value"
          :placeholder="f.value_type === 'file_path' ? '/path/to/file' : 'Value'"
        />
        <IconButton name="x" :size="13" title="删除" @click="removeMultipartField(i)" />
      </div>
      <button class="rf-btn rf-btn-sm" type="button" @click="addMultipartField">
        <Icon name="plus" :size="13" /> 添加字段
      </button>
    </div>

    <div v-else-if="activeTab === 'binary'" class="binary-box">
      <label class="binary-label">
        <Icon name="upload" :size="14" /> 文件路径
      </label>
      <input
        v-model="bodyAny.path"
        class="rf-input rf-input-sm binary-input"
        spellcheck="false"
        placeholder="/path/to/file.bin（如 /Users/me/avatar.png）"
      />
      <p class="binary-hint">
        发送时后端读取该文件的原始字节作为请求体；Content-Type 默认
        application/octet-stream，可在 Headers 标签改为实际类型（如 image/png）。
      </p>
    </div>

    <p v-else class="body-hint">该请求不携带 Body。</p>
  </div>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.mode-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.mode-tabs {
  flex: 0 1 auto;
  min-width: 0;
}

.raw-subtype {
  width: 130px;
  flex-shrink: 0;
}

.body-input {
  width: 100%;
  min-height: 120px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  resize: vertical;
}

.gql-editor {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.editor-fields {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.kv-row {
  display: flex;
  gap: 6px;
  align-items: center;
}

.kv-check {
  accent-color: var(--accent);
}

.kv-key {
  width: 220px;
}

.kv-value {
  flex: 1;
}

.mp-type {
  width: 110px;
  flex-shrink: 0;
}

.binary-box {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.binary-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-2);
}

.binary-label svg {
  color: var(--accent);
}

.binary-input {
  width: 100%;
  font-family: var(--font-mono);
}

.binary-hint {
  margin: 0;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--text-3);
}

.body-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-3);
}
</style>

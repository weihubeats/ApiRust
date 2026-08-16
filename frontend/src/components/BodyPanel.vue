<script setup lang="ts">
/**
 * BodyPanel：请求体面板（none / json / text / graphql / urlencoded / multipart）。
 * Body 编辑区只支持 none/json/text/graphql；urlencoded/multipart 为字段行编辑。
 * bodyAny 用 any 放宽联合类型访问（模板 v-model 直写 raw / spec.*）。
 */
import { computed } from 'vue'
import CustomSelect from './ui/CustomSelect.vue'
import Icon from './ui/Icon.vue'
import IconButton from './ui/IconButton.vue'
import JsonEditor from './ui/JsonEditor.vue'
import type { Endpoint, KeyValue, MultipartField } from '../types/foxApi'

const props = defineProps<{ draft: Endpoint | null }>()

const bodyAny = computed(() => props.draft?.request.body as any)
const graphql = computed(() => bodyAny.value?.spec as any)

const BODY_MODES: Array<{ value: string; label: string }> = [
  { value: 'none', label: '无 Body' },
  { value: 'json', label: 'JSON' },
  { value: 'text', label: 'Text' },
  { value: 'graphql', label: 'GraphQL' },
  { value: 'urlencoded', label: '表单 (x-www-form-urlencoded)' },
  { value: 'multipart', label: '多部件 (multipart/form-data)' },
]
const MULTIPART_TYPE_OPTIONS = [
  { value: 'text', label: '文本' },
  { value: 'file_path', label: '文件路径' },
]

/** Body 模式切换：整体替换为对应形状的默认对象（避免残留多余字段）。 */
function setBodyMode(mode: string): void {
  const req = props.draft?.request
  if (!req) return
  const prev = bodyAny.value
  switch (mode) {
    case 'none':
      req.body = { mode: 'none' }
      break
    case 'json':
    case 'text':
      req.body = { mode, raw: prev?.raw ?? '' }
      break
    case 'graphql':
      req.body = {
        mode: 'graphql',
        spec: { query: prev?.spec?.query ?? '', variables: prev?.spec?.variables ?? '{}', operation_name: prev?.spec?.operation_name ?? '' },
      }
      break
    case 'urlencoded':
      req.body = { mode: 'urlencoded', fields: prev?.fields ?? [] }
      break
    case 'multipart':
      req.body = { mode: 'multipart', fields: prev?.fields ?? [] }
      break
    default:
      req.body = { mode: 'none' }
  }
}

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
    <CustomSelect
      :model-value="bodyAny?.mode ?? 'none'"
      :options="BODY_MODES"
      size="sm"
      class="body-mode-select"
      @update:model-value="setBodyMode(String($event))"
    />
    <JsonEditor
      v-if="bodyAny?.mode === 'json'"
      v-model="bodyAny.raw"
      placeholder='{ "key": "value" }'
      :min-height="120"
    />
    <textarea
      v-else-if="bodyAny?.mode === 'text'"
      v-model="bodyAny.raw"
      class="rf-input body-input"
      spellcheck="false"
      placeholder="纯文本内容"
    ></textarea>
    <div v-else-if="bodyAny?.mode === 'graphql'" class="gql-editor">
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
    <div v-else-if="bodyAny?.mode === 'urlencoded'" class="editor-fields">
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
    <div v-else-if="bodyAny?.mode === 'multipart'" class="editor-fields">
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
    <p v-else-if="bodyAny?.mode !== 'none'" class="body-hint">暂不支持该 Body 模式。</p>
  </div>
</template>

<style scoped>
.panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.body-mode-select {
  width: 200px;
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

.body-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-3);
}
</style>
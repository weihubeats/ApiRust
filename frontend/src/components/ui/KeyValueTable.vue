<script setup lang="ts">
/**
 * KeyValueTable：键/值/描述 三列（可选启用列）表格，Postman 式自动追加行。
 * - 末尾常驻一条空白「幽灵行」（弱化显示）：在其中任一单元格输入即自动
 *   成为正式行，并立即在下方补齐新的空白行，无需点「+ 添加」；
 * - 行删除按钮仅对正式行显示（悬停显现）；中间空行失焦自动移除，只保留底部一条幽灵行；
 * - 键盘流：Tab 天然从 Key → Value；Value 内按 Enter / Tab 跳到下一行 Key，
 *   处于末尾时自动补行并聚焦。
 */
import { nextTick, ref, watch, type ComponentPublicInstance } from 'vue'
import IconButton from './IconButton.vue'

export interface KVRow {
  key?: string
  value?: string
  enabled?: boolean
  description?: string
}

const props = withDefaults(
  defineProps<{
    modelValue: KVRow[]
    showEnable?: boolean
    showDescription?: boolean
    keyPlaceholder?: string
    valuePlaceholder?: string
    descriptionPlaceholder?: string
    disabled?: boolean
  }>(),
  {
    showEnable: true,
    showDescription: true,
    keyPlaceholder: 'Key',
    valuePlaceholder: 'Value',
    descriptionPlaceholder: '描述',
    disabled: false,
  },
)

const emit = defineEmits<{ 'update:modelValue': [rows: KVRow[]] }>()

const rows = ref<KVRow[]>([])
const keyInputs = new Map<number, HTMLInputElement>()

function blank(): KVRow {
  return { key: '', value: '', enabled: true, description: '' }
}

function isEmpty(row: KVRow): boolean {
  return !row.key && !row.value
}

watch(
  () => props.modelValue,
  (v) => {
    const list = [...(v ?? [])]
    const last = list[list.length - 1]
    if (!last || !isEmpty(last)) list.push(blank())
    rows.value = list
  },
  { immediate: true, deep: true },
)

/** 上抛：过滤掉中间的空行（保留底部幽灵行）。 */
function sync(): void {
  const list = rows.value.filter((r, i) => i === rows.value.length - 1 || r.key || r.value)
  emit('update:modelValue', list)
}

function isGhost(row: KVRow): boolean {
  return isEmpty(row) && row === rows.value[rows.value.length - 1]
}

function ensureTail(): void {
  const last = rows.value[rows.value.length - 1]
  if (!isEmpty(last)) rows.value.push(blank())
  rows.value = [...rows.value]
}

function remove(index: number): void {
  rows.value.splice(index, 1)
  rows.value = [...rows.value]
  sync()
}

function onCellInput(): void {
  ensureTail()
  sync()
}

function onCellBlur(row: KVRow): void {
  const i = rows.value.indexOf(row)
  if (i === -1 || i === rows.value.length - 1) return
  if (isEmpty(row)) {
    rows.value.splice(i, 1)
    rows.value = [...rows.value]
    sync()
  }
}

function setKeyRef(i: number): (ref: Element | ComponentPublicInstance | null) => void {
  return (ref) => {
    const el = ref instanceof HTMLInputElement ? ref : null
    if (el) keyInputs.set(i, el)
    else keyInputs.delete(i)
  }
}

/** Key/Value/描述共用：仅 Value 内 Enter/Tab 跳下一行 Key；Key 内 Tab 走默认顺序（→Value）。 */
function onKeydown(event: KeyboardEvent, i: number): void {
  if (!(event.key === 'Enter' || event.key === 'Tab')) return
  if (!(event.target as HTMLElement).classList.contains('kvt-value')) return
  event.preventDefault()
  ensureTail()
  void nextTick(() => {
    const el = keyInputs.get(i + 1)
    if (!el) return
    el.focus()
    if (!el.value) el.select()
  })
}
</script>

<template>
  <div class="kvt" :class="{ disabled }">
    <div class="kvt-head">
      <span v-if="showEnable" class="kvt-col kvt-enable"></span>
      <span class="kvt-col kvt-key rf-mono">Key</span>
      <span class="kvt-col kvt-value rf-mono">Value</span>
      <span v-if="showDescription" class="kvt-col kvt-desc rf-mono">Description</span>
      <span class="kvt-col kvt-actions"></span>
    </div>

    <div
      v-for="(row, i) in rows"
      :key="i"
      class="kvt-row"
      :class="{ off: showEnable && row.enabled === false, ghost: isGhost(row) }"
    >
      <span v-if="showEnable" class="kvt-col kvt-enable">
        <input
          v-model="row.enabled"
          type="checkbox"
          class="kvt-check"
          :disabled="disabled || isGhost(row)"
          @change="sync"
        />
      </span>
      <input
        v-model="row.key"
        class="kvt-input kvt-col kvt-key rf-mono"
        :placeholder="keyPlaceholder"
        :disabled="disabled"
        spellcheck="false"
        :ref="setKeyRef(i)"
        @input="onCellInput"
        @keydown="onKeydown($event, i)"
        @blur="onCellBlur(row)"
      />
      <input
        v-model="row.value"
        class="kvt-input kvt-col kvt-value rf-mono"
        :placeholder="valuePlaceholder"
        :disabled="disabled"
        spellcheck="false"
        @input="onCellInput"
        @keydown="onKeydown($event, i)"
        @blur="onCellBlur(row)"
      />
      <input
        v-if="showDescription"
        v-model="row.description"
        class="kvt-input kvt-col kvt-desc"
        :placeholder="descriptionPlaceholder"
        :disabled="disabled"
        spellcheck="false"
        @input="onCellInput"
        @keydown="onKeydown($event, i)"
        @blur="onCellBlur(row)"
      />
      <span class="kvt-col kvt-actions">
        <IconButton
          v-if="!isGhost(row)"
          name="trash"
          :size="13"
          tone="danger"
          title="删除"
          @click="remove(i)"
        />
      </span>
    </div>
  </div>
</template>

<style scoped>
.kvt {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
  background: var(--bg-card);
}
.kvt.disabled {
  opacity: 0.5;
  pointer-events: none;
}

.kvt-head {
  display: flex;
  align-items: center;
  height: 28px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-panel);
  font-size: 10.5px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--text-3);
}

.kvt-col {
  flex-shrink: 0;
}
.kvt-enable {
  width: 36px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}
.kvt-key {
  width: 34%;
  min-width: 120px;
}
.kvt-value {
  flex: 1;
  min-width: 0;
}
.kvt-desc {
  width: 150px;
}
.kvt-actions {
  width: 32px;
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
}

.kvt-row {
  display: flex;
  align-items: center;
  min-height: 32px;
  border-bottom: 1px solid var(--border);
  transition: background var(--dur) var(--ease);
}
.kvt-row:last-child {
  border-bottom: none;
}
.kvt-row:hover {
  background: var(--bg-hover);
}
.kvt-row.off .kvt-input {
  opacity: 0.45;
}

/* 底部幽灵行：弱化占位，输入即自动成为正式行 */
.kvt-row.ghost .kvt-input {
  opacity: 0.5;
}
.kvt-row.ghost .kvt-input:focus {
  opacity: 1;
}

.kvt-check {
  accent-color: var(--accent);
  cursor: pointer;
}

.kvt-input {
  height: 32px;
  border: none;
  background: transparent;
  color: var(--text-1);
  font-size: 12px;
  outline: none;
  padding: 0 8px;
  min-width: 0;
}
.kvt-input::placeholder {
  color: var(--text-3);
}
.kvt-input:focus {
  background: var(--bg-elevated);
  box-shadow: inset 0 0 0 1px var(--accent);
}

.kvt-actions {
  opacity: 0;
  transition: opacity var(--dur) var(--ease);
}
.kvt-row:hover .kvt-actions,
.kvt-row:focus-within .kvt-actions {
  opacity: 1;
}
</style>
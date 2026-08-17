<script setup lang="ts">
/**
 * JsonEditor：JSON 编辑区（暗色代码编辑器风格）。
 * - 覆盖层方案：透明 textarea 叠加在高亮 <pre> 上（零依赖，滚动同步）；
 * - 左侧行号栏：随内容垂直平移、细边框与正文分隔；
 * - 右上角浮动半透明工具条：校验状态 + JSON 格式下拉（美化/紧凑）+ 格式化按钮；
 * - 深色底色 #121318，聚焦时 1px 紫色光晕（原 3px 重描边移除）。
 */
import { computed, nextTick, ref, watch } from 'vue'
import { useToast } from '../../composables/useToast'
import { highlightJSON } from '../../utils/highlight'
import { compactJson, prettyJson } from '../../utils/jsonFormat'
import { JsonFormatError } from '../../utils/jsonFormat'
import CustomSelect from './CustomSelect.vue'
import Icon from './Icon.vue'

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    /** 编辑区最小高度（px）。 */
    minHeight?: number
  }>(),
  { placeholder: '', minHeight: 120 },
)

const emit = defineEmits<{ (e: 'update:modelValue', value: string): void }>()

const toast = useToast()
const taRef = ref<HTMLTextAreaElement | null>(null)
const preRef = ref<HTMLElement | null>(null)
const scrollTop = ref(0)

/** 空内容渲染一个空格，保证 pre 与 textarea 高度一致（滚动同步前提）。 */
const html = computed(() => highlightJSON(props.modelValue.length ? props.modelValue : ' '))

const lineCount = computed(() => props.modelValue.split('\n').length)

/** 行号栏宽度随位数增长：左留白 + 位数×字宽 + 右留白。 */
const gutterWidth = computed(() => 10 + String(lineCount.value).length * 8 + 10)

const status = computed<'empty' | 'ok' | 'invalid'>(() => {
  const t = props.modelValue.trim()
  if (!t) return 'empty'
  try {
    JSON.parse(t)
    return 'ok'
  } catch {
    return 'invalid'
  }
})

// ---------- 格式类型 ----------
type JsonFormatMode = 'pretty' | 'compact'
const formatMode = ref<JsonFormatMode>('pretty')
const FORMAT_OPTIONS: Array<{ value: string; label: string }> = [
  { value: 'pretty', label: 'JSON' },
  { value: 'compact', label: 'JSON 紧凑' },
]

function onInput(e: Event): void {
  emit('update:modelValue', (e.target as HTMLTextAreaElement).value)
}

function syncScroll(e: Event): void {
  const ta = e.target as HTMLTextAreaElement
  scrollTop.value = ta.scrollTop
  if (preRef.value) {
    preRef.value.scrollTop = ta.scrollTop
    preRef.value.scrollLeft = ta.scrollLeft
  }
}

// 内容变化（格式化 / 粘贴 / 回退）后重新对齐滚动，避免高亮层与行号错位。
watch(
  () => props.modelValue,
  () => {
    if (taRef.value && preRef.value) {
      preRef.value.scrollTop = taRef.value.scrollTop
      preRef.value.scrollLeft = taRef.value.scrollLeft
      scrollTop.value = taRef.value.scrollTop
    }
  },
)

function format(): void {
  // 以 textarea 的实时 DOM 值为权威来源：真实浏览器里 input 事件可能丢失或
  // 延迟（拖拽写入、自动填充、受控回写竞态），导致 props.modelValue 落后于
  // 用户实际编辑的内容——此时若按 props 格式化会把编辑器回退成旧数据。
  const t = (taRef.value?.value ?? props.modelValue).trim()
  if (!t) return
  try {
    // 无损格式化：保留重复键、键顺序与数字原文（parse/stringify 往返会丢重复键）。
    const out = formatMode.value === 'compact' ? compactJson(t) : prettyJson(t)
    if (out !== props.modelValue) {
      emit('update:modelValue', out)
    } else if (taRef.value && taRef.value.value !== out) {
      // 模型已一致但 DOM 残留未同步的值：直接纠正 DOM。
      taRef.value.value = out
    }
    toast.success('已格式化')
    void nextTick(() => {
      // 双保险：emit 回写后强制对齐 DOM（WebKit 偶发不回写受控 value）。
      if (taRef.value && taRef.value.value !== props.modelValue) {
        taRef.value.value = props.modelValue
      }
      taRef.value?.focus()
    })
  } catch (err) {
    toast.error(err instanceof JsonFormatError ? `JSON 无效：${err.message}` : 'JSON 无效，无法格式化')
  }
}
</script>

<template>
  <div class="json-editor">
    <div class="hl-wrap" :style="{ minHeight: `${minHeight}px` }">
      <div class="hl-gutter" :style="{ width: `${gutterWidth}px` }" aria-hidden="true">
        <div
          class="hl-gutter-inner"
          :style="{ transform: `translateY(${-scrollTop}px)` }"
        >
          <div v-for="n in lineCount" :key="n" class="hl-gutter-line">{{ n }}</div>
        </div>
      </div>
      <pre
        ref="preRef"
        class="hl-pre"
        aria-hidden="true"
        v-html="html"
        :style="{ paddingLeft: `${gutterWidth}px` }"
      ></pre>
      <textarea
        ref="taRef"
        class="hl-ta"
        :value="modelValue"
        :placeholder="placeholder"
        spellcheck="false"
        :style="{ paddingLeft: `${gutterWidth}px` }"
        @input="onInput"
        @change="onInput"
        @scroll="syncScroll"
      ></textarea>

      <div class="hl-float">
        <span
          v-if="status !== 'empty'"
          class="hl-status"
          :class="{ invalid: status === 'invalid' }"
          :title="status === 'invalid' ? 'JSON 无效' : 'JSON 有效'"
        >
          <span class="hl-dot"></span>{{ status === 'invalid' ? '无效' : '有效' }}
        </span>
        <CustomSelect
          :model-value="formatMode"
          :options="FORMAT_OPTIONS"
          size="sm"
          class="hl-mode"
          @update:model-value="formatMode = String($event) as JsonFormatMode"
        />
        <button class="hl-btn" type="button" title="格式化 JSON" @click="format">
          <Icon name="zap" :size="12" />
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.json-editor {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  min-height: 0;
}

.hl-wrap {
  position: relative;
  border: 1px solid rgba(255, 255, 255, 0.09);
  border-radius: 8px;
  background: #121318;
  overflow: hidden;
  flex: 1;
  min-height: 0;
  transition:
    border-color var(--dur) var(--ease),
    box-shadow var(--dur) var(--ease);
}

/* 1px 紫色聚焦光晕（替代原 3px 重描边） */
.hl-wrap:focus-within {
  border-color: rgba(168, 85, 247, 0.6);
  box-shadow: 0 0 0 1px rgba(168, 85, 247, 0.5);
}

.hl-pre,
.hl-ta {
  margin: 0;
  padding: 10px 12px;
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.55;
  white-space: pre;
  tab-size: 2;
}

.hl-pre {
  position: absolute;
  inset: 0;
  color: #d7dbe3;
  pointer-events: none;
  overflow: hidden;
  word-break: normal;
}

.hl-ta {
  position: relative;
  width: 100%;
  height: 100%;
  display: block;
  box-sizing: border-box;
  resize: vertical;
  border: none;
  outline: none;
  background: transparent;
  color: transparent;
  caret-color: #c084fc;
  overflow: auto;
}
.hl-ta::placeholder {
  color: rgba(215, 219, 227, 0.35);
}
.hl-ta::selection {
  background: rgba(168, 85, 247, 0.28);
}

/* ---- 行号栏 ---- */
.hl-gutter {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  z-index: 1;
  background: rgba(255, 255, 255, 0.03);
  border-right: 1px solid rgba(255, 255, 255, 0.07);
  user-select: none;
  pointer-events: none;
}

.hl-gutter-inner {
  padding-top: 10px;
  will-change: transform;
}

.hl-gutter-line {
  height: calc(12.5px * 1.55);
  line-height: calc(12.5px * 1.55);
  text-align: right;
  padding-right: 10px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--tok-gutter);
}

/* ---- 右上角浮动工具条 ---- */
.hl-float {
  position: absolute;
  top: 8px;
  right: 8px;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  background: rgba(18, 19, 24, 0.6);
  backdrop-filter: blur(8px);
  -webkit-backdrop-filter: blur(8px);
}

.hl-status {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 0 6px;
  font-size: 11px;
  color: #86efac;
  white-space: nowrap;
}
.hl-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
  flex-shrink: 0;
}
.hl-status.invalid {
  color: #f87171;
}

.hl-mode :deep(.cs-trigger) {
  height: 24px;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  box-shadow: none;
  color: #d7dbe3;
  font-size: 11.5px;
}
.hl-mode :deep(.cs-trigger:hover) {
  background: rgba(255, 255, 255, 0.12);
}
.hl-mode :deep(.cs-caret) {
  color: rgba(215, 219, 227, 0.6);
}

.hl-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  color: #d7dbe3;
  font-size: 11.5px;
  font-family: inherit;
  cursor: pointer;
  transition: background var(--dur) var(--ease);
}
.hl-btn:hover {
  background: rgba(255, 255, 255, 0.12);
}

/* ---- 统一 JSON 语法着色（--tok-*，与响应 Body 共用，见 constants/editorTheme.ts） ---- */
:deep(.hl-k) {
  color: var(--tok-key);
  font-weight: 600;
}
:deep(.hl-s) {
  color: var(--tok-str);
}
:deep(.hl-n) {
  color: var(--tok-num);
}
:deep(.hl-b) {
  color: var(--tok-bool);
}
:deep(.hl-null) {
  color: var(--tok-null);
  font-style: italic;
}
:deep(.hl-p) {
  color: var(--tok-punct);
}
</style>

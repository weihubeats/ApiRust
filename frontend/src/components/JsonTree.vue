<script setup lang="ts">
/**
 * JsonTree：JSON 树形查看器（响应体 Pretty 视图）。
 *
 * - 扁平行渲染：每行带行号 + 缩进 + 折叠箭头，容器节点（对象/数组）可折叠/展开；
 * - 语法着色：键 / 字符串 / 数字 / 布尔 / null / 标点；
 * - 默认展开前 expandDepth 层，更深自动折叠（大响应体友好）；
 * - 长字符串截断展示，悬浮显示全文。
 */
import { computed, reactive } from 'vue'
import { escapeHtml } from '../utils/highlight'
import Icon from './ui/Icon.vue'

interface Line {
  depth: number
  html: string
  title?: string
  toggleable?: string
  open?: boolean
}

const props = withDefaults(
  defineProps<{
    data: unknown
    expandDepth?: number
  }>(),
  { expandDepth: 3 },
)

/** 折叠状态：path（`$["key"]` / `$[0]`）→ 是否展开。 */
const expanded = reactive<Record<string, boolean>>({})

function toggle(path: string): void {
  expanded[path] = !expanded[path]
}

function tok(text: string, cls: string): string {
  return `<span class="jt-tok jt-${cls}">${escapeHtml(text)}</span>`
}

function keyToken(key: string): string {
  return `${tok(JSON.stringify(key), 'key')}${tok(': ', 'punct')}`
}

function leafHtml(value: unknown): string {
  if (typeof value === 'string') {
    const truncated = value.length > 160 ? `${value.slice(0, 160)}…` : value
    return tok(JSON.stringify(truncated), 'str')
  }
  if (typeof value === 'number') return tok(String(value), 'num')
  if (typeof value === 'boolean') return tok(String(value), 'bool')
  return tok('null', 'null')
}

const lines = computed<Line[]>(() => {
  const out: Line[] = []

  function walk(
    value: unknown,
    depth: number,
    path: string,
    keyHtml: string | null,
    isLast: boolean,
  ): void {
    if (value === null || typeof value !== 'object') {
      out.push({
        depth,
        html: `${keyHtml ?? ''}${leafHtml(value)}${isLast ? '' : tok(',', 'punct')}`,
        title: typeof value === 'string' ? value : undefined,
      })
      return
    }

    const isArray = Array.isArray(value)
    const count = isArray ? (value as unknown[]).length : Object.keys(value as object).length

    if (count === 0) {
      out.push({
        depth,
        html: `${keyHtml ?? ''}${tok(isArray ? '[]' : '{}', 'punct')}${isLast ? '' : tok(',', 'punct')}`,
      })
      return
    }

    const open = expanded[path] ?? depth < props.expandDepth
    if (!open) {
      out.push({
        depth,
        html: `${keyHtml ?? ''}${tok(isArray ? '[' : '{', 'punct')}${tok(' … ', 'dots')}${tok(`${count} 项`, 'meta')}${tok(isArray ? ']' : '}', 'punct')}${isLast ? '' : tok(',', 'punct')}`,
        toggleable: path,
        open: false,
      })
      return
    }

    out.push({
      depth,
      html: `${keyHtml ?? ''}${tok(isArray ? '[' : '{', 'punct')}`,
      toggleable: path,
      open: true,
    })

    if (isArray) {
      const arr = value as unknown[]
      for (let i = 0; i < arr.length; i++) {
        walk(arr[i], depth + 1, `${path}[${i}]`, null, i === arr.length - 1)
      }
    } else {
      const entries = Object.entries(value as Record<string, unknown>)
      for (let i = 0; i < entries.length; i++) {
        const [k, v] = entries[i]
        walk(v, depth + 1, `${path}[${JSON.stringify(k)}]`, keyToken(k), i === entries.length - 1)
      }
    }

    out.push({
      depth,
      html: `${tok(isArray ? ']' : '}', 'punct')}${isLast ? '' : tok(',', 'punct')}`,
    })
  }

  if (props.data !== undefined) walk(props.data, 0, '$', null, true)
  return out
})
</script>

<template>
  <div class="jt">
    <div
      v-for="(line, i) in lines"
      :key="i"
      class="jt-line"
      :class="{ 'has-toggle': line.toggleable }"
      :title="line.title"
      :style="{ paddingLeft: `${line.depth * 16}px` }"
    >
      <span class="jt-gutter">{{ i + 1 }}</span>
      <button
        v-if="line.toggleable"
        type="button"
        class="jt-toggle"
        :class="{ open: line.open }"
        :aria-label="line.open ? '折叠' : '展开'"
        @click="toggle(line.toggleable)"
      >
        <Icon :name="line.open ? 'chevron-down' : 'chevron-right'" :size="12" />
      </button>
      <span class="jt-code" v-html="line.html"></span>
    </div>
  </div>
</template>

<style scoped>
.jt {
  font-family: var(--font-mono);
  font-size: 12.5px;
  line-height: 1.55;
}

.jt-line {
  display: flex;
  align-items: center;
  min-width: 0;
  white-space: pre;
  color: var(--text-1);
}

.jt-gutter {
  flex-shrink: 0;
  width: 38px;
  text-align: right;
  padding-right: 10px;
  user-select: none;
  color: var(--text-3);
  font-size: 11px;
  opacity: 0.7;
}

.jt-toggle {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  margin: 0;
  padding: 0;
  border: none;
  border-radius: 4px;
  background: none;
  color: var(--text-2);
  cursor: pointer;
  transition:
    background var(--dur) var(--ease),
    color var(--dur) var(--ease);
}
.jt-toggle:hover {
  background: var(--bg-hover);
  color: var(--text-1);
}
.jt-toggle.open {
  color: var(--accent);
}

.jt-code {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 语法着色（VS Code 深色主题调色板）。
 * 行内 token 由 v-html 动态注入，无 scoped 属性，需用 :deep 穿透。 */
:deep(.jt-tok.jt-key) {
  color: #569cd6;
}
:deep(.jt-tok.jt-str) {
  color: #ce9178;
}
:deep(.jt-tok.jt-num) {
  color: #4fc1ff;
}
:deep(.jt-tok.jt-bool) {
  color: #b5cea8;
}
:deep(.jt-tok.jt-null) {
  color: #6a9955;
  font-style: italic;
}
:deep(.jt-tok.jt-punct) {
  color: #d4d4d4;
}
:deep(.jt-tok.jt-dots) {
  color: #888;
  font-style: italic;
}
:deep(.jt-tok.jt-meta) {
  color: #888;
  font-style: italic;
  font-size: 11px;
}
</style>

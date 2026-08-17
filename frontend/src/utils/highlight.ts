/**
 * highlight.ts：轻量语法高亮工具（正则分词，无第三方依赖）。
 *
 * - escapeHtml / highlightJSON / highlightGraphQL 从 GraphQLView.vue 提取为共享实现；
 * - 输出 HTML 片段，配合调用方作用域内的 .hl-* 颜色类（.hl-s/.hl-k/.hl-n/.hl-b/.hl-c/.hl-v/.hl-p）。
 */

/** HTML 转义（所有高亮输出必须先转义再包 span）。 */
export function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

const GRAPHQL_KEYWORDS =
  'query|mutation|subscription|fragment|on|schema|scalar|type|interface|union|enum|input|implements|directive|extend|true|false|null'

/** 轻量 GraphQL 高亮（注释 / 字符串 / 变量 / 关键字 / 数字）。 */
export function highlightGraphQL(code: string): string {
  const re =
    /(#[^\n]*)|("(?:[^"\\]|\\.)*"|"""(?:.|\n)*?""")|(\$[A-Za-z_][A-Za-z0-9_]*)|([A-Za-z_][A-Za-z0-9_]*)|(-?\b\d+(?:\.\d+)?\b)|([{}\[\]():!=\|&,.])/g
  let out = ''
  let last = 0
  for (const m of code.matchAll(re)) {
    out += escapeHtml(code.slice(last, m.index))
    const [full, comment, str, variable, ident, num] = m
    if (comment) out += `<span class="hl-c">${escapeHtml(full)}</span>`
    else if (str) out += `<span class="hl-s">${escapeHtml(full)}</span>`
    else if (variable) out += `<span class="hl-v">${escapeHtml(full)}</span>`
    else if (ident) {
      if (GRAPHQL_KEYWORDS.split('|').includes(ident)) {
        out += `<span class="hl-k">${escapeHtml(full)}</span>`
      } else {
        out += escapeHtml(full)
      }
    } else if (num) out += `<span class="hl-n">${escapeHtml(full)}</span>`
    else out += `<span class="hl-p">${escapeHtml(full)}</span>`
    last = m.index! + full.length
  }
  out += escapeHtml(code.slice(last))
  return out
}

/** JSON 词法片段：文本 + 着色类（'' 表示不着色）。 */
export interface JsonToken {
  text: string
  cls: string
}

const JSON_RE =
  /("(?:[^"\\]|\\.)*")(\s*:)?|(-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b)|(true|false)|(\bnull\b)|([{}\[\],])/g

/** JSON 分词：键 / 字符串 / 数字 / 布尔 / null / 标点（请求编辑器与响应视图共用）。 */
export function jsonTokens(code: string): JsonToken[] {
  const out: JsonToken[] = []
  let last = 0
  for (const m of code.matchAll(JSON_RE)) {
    const head = code.slice(last, m.index)
    if (head) out.push({ text: head, cls: '' })
    const [full, str, colon, num, bool, nul, punct] = m
    if (str) out.push({ text: full, cls: colon ? 'hl-k' : 'hl-s' })
    else if (num) out.push({ text: full, cls: 'hl-n' })
    else if (bool) out.push({ text: full, cls: 'hl-b' })
    else if (nul) out.push({ text: full, cls: 'hl-null' })
    else if (punct) out.push({ text: full, cls: 'hl-p' })
    last = m.index! + full.length
  }
  const tail = code.slice(last)
  if (tail) out.push({ text: tail, cls: '' })
  return out
}

/** 轻量 JSON 高亮（键 / 字符串 / 数字 / 布尔 / null / 标点）。 */
export function highlightJSON(code: string): string {
  return jsonTokens(code)
    .map((t) => (t.cls ? `<span class="${t.cls}">${escapeHtml(t.text)}</span>` : escapeHtml(t.text)))
    .join('')
}

/** JSON 高亮 + 查找标记（响应行视图用；空 query 时退化为纯高亮）。 */
export function highlightJSONText(code: string, query: string): string {
  if (!query) return highlightJSON(code)
  const ql = query.toLowerCase()
  const lower = code.toLowerCase()
  let out = ''
  let from = 0
  for (;;) {
    const idx = lower.indexOf(ql, from)
    if (idx === -1) break
    out += highlightJSON(code.slice(from, idx))
    out += `<mark class="rp-find-mark">${escapeHtml(code.slice(idx, idx + query.length))}</mark>`
    from = idx + query.length
  }
  out += highlightJSON(code.slice(from))
  return out
}

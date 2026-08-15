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

/** 轻量 JSON 高亮（键 / 字符串 / 数字 / 布尔 / null）。 */
export function highlightJSON(code: string): string {
  const re =
    /("(?:[^"\\]|\\.)*")(\s*:)?|(-?\b\d+(?:\.\d+)?(?:[eE][+-]?\d+)?\b)|(true|false)|(\bnull\b)/g
  let out = ''
  let last = 0
  for (const m of code.matchAll(re)) {
    out += escapeHtml(code.slice(last, m.index))
    const [full, str, colon, num, bool, nul] = m
    if (str) out += `<span class="hl-s${colon ? ' hl-k' : ''}">${escapeHtml(full)}</span>`
    else if (num) out += `<span class="hl-n">${escapeHtml(full)}</span>`
    else if (bool) out += `<span class="hl-b">${escapeHtml(full)}</span>`
    else if (nul) out += `<span class="hl-null">${escapeHtml(full)}</span>`
    last = m.index! + full.length
  }
  out += escapeHtml(code.slice(last))
  return out
}

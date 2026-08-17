/**
 * codeImport：从各语言 HTTP 客户端代码片段解析请求（cURL 之外的前端解析器）。
 *
 * 输出与后端 parse_curl_command 的 CurlParsed 同形状，导入路径复用
 * store.openCurlDraft。解析为启发式最佳努力：覆盖常见写法
 * （JS fetch/axios、Python requests、Java OkHttp/HttpURLConnection、Go net/http），
 * 无法识别的部分（变量引用等）跳过而不是报错；仅「找不到 URL」视为失败。
 */
import type { AuthSpec, BodySpec, CurlParsed, HttpMethod, KeyValue } from '../types/foxApi'

export type SnippetLang = 'curl' | 'java' | 'python' | 'javascript' | 'go'

export const SNIPPET_LANGS: Array<{ value: SnippetLang | 'auto'; label: string }> = [
  { value: 'auto', label: '自动检测' },
  { value: 'curl', label: 'cURL' },
  { value: 'java', label: 'Java (OkHttp / HttpURLConnection)' },
  { value: 'python', label: 'Python (requests)' },
  { value: 'javascript', label: 'JavaScript (fetch / axios)' },
  { value: 'go', label: 'Go (net/http)' },
]

const METHODS: HttpMethod[] = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']

function kv(key: string, value: string): KeyValue {
  return { key, value, enabled: true, description: '' }
}

// ---------- 通用工具 ----------

/** 提取第一个 http(s) URL（停在引号/空白/反引号/右括号处，去掉尾部标点）。 */
function findUrl(src: string): string | null {
  const m = src.match(/https?:\/\/[^\s"'`\\)\]}<>]+/i)
  return m ? m[0].replace(/[.,;:]+$/, '') : null
}

/** 解码字符串字面量里的常见转义（\n \t \r \" \' \\ \/）。 */
function unescapeLiteral(s: string): string {
  return s
    .replace(/\\n/g, '\n')
    .replace(/\\t/g, '\t')
    .replace(/\\r/g, '\r')
    .replace(/\\"/g, '"')
    .replace(/\\'/g, "'")
    .replace(/\\`/g, '`')
    .replace(/\\\//g, '/')
    .replace(/\\\\/g, '\\')
}

/** 匹配一段带引号的字符串字面量（含转义），返回内容；quote 为起始引号。 */
function readQuoted(src: string, start: number, quote: string): { text: string; end: number } | null {
  if (src[start] !== quote) return null
  let raw = ''
  for (let i = start + 1; i < src.length; i += 1) {
    const ch = src[i]
    if (ch === '\\' && i + 1 < src.length) {
      raw += ch + src[i + 1]
      i += 1
      continue
    }
    if (ch === quote) return { text: unescapeLiteral(raw), end: i + 1 }
    if (ch === '\n' && quote !== '`') return null
    raw += ch
  }
  return null
}

/** 从 fromIndex 起读取一个「值」：字符串字面量或平衡花括号对象字面量。 */
function readValue(src: string, from: number): { text: string; end: number; quoted: boolean } | null {
  let i = from
  while (i < src.length && /\s/.test(src[i])) i += 1
  const ch = src[i]
  if (ch === '"' || ch === "'" || ch === '`') {
    const lit = readQuoted(src, i, ch)
    if (lit) return { text: lit.text, end: lit.end, quoted: true }
    return null
  }
  if (ch === '{' || ch === '[') {
    const close = ch === '{' ? '}' : ']'
    let depth = 0
    let inStr: string | null = null
    for (let j = i; j < src.length; j += 1) {
      const c = src[j]
      if (inStr) {
        if (c === '\\') j += 1
        else if (c === inStr) inStr = null
        continue
      }
      if (c === '"' || c === "'" || c === '`') inStr = c
      else if (c === ch) depth += 1
      else if (c === close) {
        depth -= 1
        if (depth === 0) return { text: src.slice(i, j + 1), end: j + 1, quoted: false }
      }
    }
  }
  return null
}

/** 在 `key:` / `key =` 后读取值（JS/Python 对象与命名参数通用）。 */
function valueAfterKey(src: string, key: string): { text: string; quoted: boolean } | null {
  const re = new RegExp(`\\b${key}\\s*[:=]\\s*`)
  const m = re.exec(src)
  if (!m) return null
  const v = readValue(src, m.index + m[0].length)
  return v ? { text: v.text, quoted: v.quoted } : null
}

/** 解析对象/字典字面量里的 `键: 值` 字符串对（值仅取字符串字面量）。 */
function parseObjectPairs(objSrc: string): Array<[string, string]> {
  const pairs: Array<[string, string]> = []
  const re = /["'`]?([\w.$-]+)["'`]?\s*:\s*(?:"((?:[^"\\]|\\.)*)"|'((?:[^'\\]|\\.)*)'|`((?:[^`\\]|\\.)*)`)/g
  let m: RegExpExecArray | null
  while ((m = re.exec(objSrc)) !== null) {
    const value = m[2] ?? m[3] ?? m[4] ?? ''
    pairs.push([m[1], unescapeLiteral(value)])
  }
  return pairs
}

/** 按内容推断 body 模式：JSON 外形 → json；k=v 串 + urlencoded 头 → urlencoded 字段。 */
function inferBody(raw: string, contentType: string): BodySpec {
  const trimmed = raw.trim()
  const ct = contentType.toLowerCase()
  if ((ct.includes('json') || /^[[{]/.test(trimmed)) && looksLikeJson(trimmed)) {
    return { mode: 'json', raw: trimmed }
  }
  if (ct.includes('urlencoded') && trimmed.includes('=')) {
    const fields = trimmed
      .split('&')
      .map((part) => {
        const eq = part.indexOf('=')
        return eq > 0
          ? kv(decodeURIComponent(part.slice(0, eq)), decodeURIComponent(part.slice(eq + 1).replace(/\+/g, ' ')))
          : null
      })
      .filter((x): x is KeyValue => x !== null)
    if (fields.length) return { mode: 'urlencoded', fields }
  }
  return { mode: 'text', raw }
}

function looksLikeJson(s: string): boolean {
  if (!s.startsWith('{') && !s.startsWith('[')) return false
  try {
    JSON.parse(s)
    return true
  } catch {
    return false
  }
}

// ---------- 语言检测 ----------

/** 按代码特征检测语言（cURL 优先，避免与 JS 里的 fetch 混淆）。 */
export function detectLang(src: string): SnippetLang | null {
  if (/^\s*curl\b/i.test(src) || /\bcurl\s+(-[A-Za-z]|https?:\/\/)/.test(src)) return 'curl'
  if (/new Request\.Builder\(|OkHttpClient|HttpURLConnection|openConnection\(\)/.test(src)) return 'java'
  if (/\brequests\s*[.(]\s*(get|post|put|patch|delete|head|request)\b/.test(src) || /^import requests\b/m.test(src)) return 'python'
  if (/\bfetch\s*\(|\baxios\s*[.(]|XMLHttpRequest/.test(src)) return 'javascript'
  if (/http\.NewRequest|["']net\/http["']/.test(src)) return 'go'
  return null
}

function detectMethod(src: string): HttpMethod {
  // 显式 method 配置优先：method: 'POST' / setRequestMethod / NewRequest / request("POST")
  const explicit = [
    /\bmethod\s*[:=]\s*['"`](GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)['"`]/i,
    /\bsetRequestMethod\s*\(\s*['"`](GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)['"`]\s*\)/i,
    /\bhttp\.NewRequest\s*\(\s*['"`](GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)['"`]/i,
    /\brequests\.request\s*\(\s*['"`](GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)['"`]/i,
    /\baxios\s*\(\s*\{[\s\S]*?\bmethod\s*:\s*['"`](GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)['"`]/i,
  ]
  for (const re of explicit) {
    const m = re.exec(src)
    if (m) return m[1].toUpperCase() as HttpMethod
  }
  // 链式/速记调用：.post( / axios.post( / requests.post(
  const shorthand = new RegExp(
    `(?:\\.|\\b)(get|post|put|patch|delete|head)\\s*\\(`,
    'i',
  )
  const m = shorthand.exec(src)
  // 注意：Java 里 map.put/get 等链式调用可能误报，这里接受最佳努力结果，
  // 预览界面可见方法名，用户可在导入前修正。
  if (m && METHODS.includes(m[1].toUpperCase() as HttpMethod)) {
    return m[1].toUpperCase() as HttpMethod
  }
  return 'GET'
}

// ---------- 各语言解析 ----------

type PartialParsed = {
  url: string
  method: HttpMethod
  headers: KeyValue[]
  bodyRaw: string | null
  bodyFromObject: string | null
}

function parseHeadersObject(src: string, key: string): KeyValue[] {
  const re = new RegExp(`\\b${key}\\s*[:=]\\s*`)
  const m = re.exec(src)
  if (!m) return []
  const v = readValue(src, m.index + m[0].length)
  if (!v || v.quoted) return []
  return parseObjectPairs(v.text).map(([k, val]) => kv(k, val))
}

function parseJavaScript(src: string): PartialParsed {
  const url = findUrl(src)
  if (!url) throw new Error('未能从代码中识别出 http(s) URL')
  const headers = parseHeadersObject(src, 'headers')
  // body: '字面量' / data: {...}（axios 配置式）/ body: JSON.stringify({...}) → 取内层对象
  let bodyRaw: string | null = null
  let bodyFromObject: string | null = null
  const bodyAssign = /\b(?:body|data)\s*:\s*/g
  let m: RegExpExecArray | null
  while ((m = bodyAssign.exec(src)) !== null) {
    const rest = src.slice(m.index + m[0].length)
    const stringify = /^\s*JSON\.stringify\s*\(\s*/.exec(rest)
    if (stringify) {
      const inner = readValue(rest, stringify[0].length)
      if (inner && !inner.quoted) {
        bodyFromObject = inner.text
        break
      }
    } else {
      const v = readValue(rest, 0)
      if (v?.quoted) {
        bodyRaw = v.text
        break
      }
      if (v && !v.quoted) {
        bodyFromObject = v.text
        break
      }
    }
  }
  // axios.post(url, data, …)：跳过第一个参数后取字面量参数作为 body
  if (bodyRaw === null && bodyFromObject === null) {
    const call = /\baxios\s*\.\s*\w+\s*\(\s*[^,()]*\s*,\s*/.exec(src)
    if (call) {
      const v = readValue(src, call.index + call[0].length)
      if (v?.quoted) bodyRaw = v.text
      else if (v) bodyFromObject = v.text
    }
  }
  return { url, method: detectMethod(src), headers, bodyRaw, bodyFromObject }
}

function parsePython(src: string): PartialParsed {
  const url = findUrl(src)
  if (!url) throw new Error('未能从代码中识别出 http(s) URL')
  const headers = parseHeadersObject(src, 'headers')
  let bodyRaw: string | null = null
  let bodyFromObject: string | null = null
  // json={...} / json=json.dumps({...}) → JSON body
  const jsonAssign = /\bjson\s*=\s*/g
  let m: RegExpExecArray | null
  while ((m = jsonAssign.exec(src)) !== null) {
    const rest = src.slice(m.index + m[0].length)
    const dumps = /^\s*json\.dumps\s*\(\s*/.exec(rest)
    const probe = dumps ? rest.slice(dumps[0].length) : rest
    const v = readValue(probe, 0)
    if (v && !v.quoted) {
      bodyFromObject = v.text
      break
    }
  }
  // data='k=v&…'（仅当没有 json= 时）
  if (bodyFromObject === null) {
    const data = valueAfterKey(src, 'data')
    if (data?.quoted) bodyRaw = data.text
  }
  return { url, method: detectMethod(src), headers, bodyRaw, bodyFromObject }
}

function parseJava(src: string): PartialParsed {
  const url = findUrl(src)
  if (!url) throw new Error('未能从代码中识别出 http(s) URL')
  const headers: KeyValue[] = []
  const headerRe =
    /(?:\.addHeader|\.header|\.setRequestProperty|\.addRequestProperty)\s*\(\s*("(?:[^"\\]|\\.)*")\s*,\s*("(?:[^"\\]|\\.)*")\s*\)/g
  let m: RegExpExecArray | null
  while ((m = headerRe.exec(src)) !== null) {
    const key = unescapeLiteral(m[1].slice(1, -1))
    const value = unescapeLiteral(m[2].slice(1, -1))
    if (key) headers.push(kv(key, value))
  }
  // RequestBody.create：两种参数序都支持——
  //   OkHttp 3.x: create("body", MediaType.parse("mime"))
  //   OkHttp 4.x: create(mediaType, "body")，mediaType 可为变量或内联 MediaType.parse
  let bodyRaw: string | null = null
  const mediaVars = new Map<string, string>()
  const varRe = /([A-Za-z_]\w*)\s*=\s*MediaType\.parse\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)/g
  let vm: RegExpExecArray | null
  while ((vm = varRe.exec(src)) !== null) {
    mediaVars.set(vm[1], unescapeLiteral(vm[2]))
  }
  const rbRe = /RequestBody\s*\.\s*create\s*\(/g
  let rb: RegExpExecArray | null
  while ((rb = rbRe.exec(src)) !== null && bodyRaw === null) {
    const args = src.slice(rb.index + rb[0].length)
    const litFirst = readValue(args, 0)
    if (litFirst?.quoted) {
      const rest = args.slice(litFirst.end).replace(/^\s*,\s*/, '')
      if (/^MediaType/.test(rest)) {
        const inline = /MediaType\.parse\s*\(\s*"((?:[^"\\]|\\.)*)"/.exec(rest)
        if (inline) headers.push(kv('Content-Type', unescapeLiteral(inline[1])))
        bodyRaw = litFirst.text
      }
      continue
    }
    let mime: string | null = null
    let afterMedia = -1
    const inline = /^MediaType\.parse\s*\(\s*"((?:[^"\\]|\\.)*)"\s*\)\s*,/.exec(args)
    if (inline) {
      mime = unescapeLiteral(inline[1])
      afterMedia = inline[0].length
    } else {
      const idm = /^\s*([A-Za-z_]\w*)\s*,/.exec(args)
      if (idm && mediaVars.has(idm[1])) {
        mime = mediaVars.get(idm[1]) ?? null
        afterMedia = idm[0].length
      }
    }
    if (mime !== null && afterMedia >= 0) {
      const body = readValue(args, afterMedia)
      if (body?.quoted) {
        headers.push(kv('Content-Type', mime))
        bodyRaw = body.text
      }
    }
  }
  // HttpURLConnection: conn.getOutputStream().write("...".getBytes())
  if (bodyRaw === null) {
    const write = /\.write\s*\(\s*("(?:[^"\\]|\\.)*")\s*\.getBytes/.exec(src)
    if (write) bodyRaw = unescapeLiteral(write[1].slice(1, -1))
  }
  return { url, method: detectMethod(src), headers, bodyRaw, bodyFromObject: null }
}

function parseGo(src: string): PartialParsed {
  const url = findUrl(src)
  if (!url) throw new Error('未能从代码中识别出 http(s) URL')
  const headers: KeyValue[] = []
  const setRe = /req\.Header\.(?:Set|Add)\s*\(\s*("(?:[^"\\]|\\.)*")\s*,\s*("(?:[^"\\]|\\.)*")\s*\)/g
  let m: RegExpExecArray | null
  while ((m = setRe.exec(src)) !== null) {
    headers.push(kv(unescapeLiteral(m[1].slice(1, -1)), unescapeLiteral(m[2].slice(1, -1))))
  }
  // bytes.NewBufferString("…") / strings.NewReader("…")
  let bodyRaw: string | null = null
  const bodyRe = /(?:bytes\.NewBufferString|strings\.NewReader)\s*\(\s*("(?:[^"\\]|\\.)*")/g
  let b: RegExpExecArray | null
  while ((b = bodyRe.exec(src)) !== null) {
    bodyRaw = unescapeLiteral(b[1].slice(1, -1))
    break
  }
  return { url, method: detectMethod(src), headers, bodyRaw, bodyFromObject: null }
}

// ---------- 汇总 ----------

const PARSERS: Record<Exclude<SnippetLang, 'curl'>, (src: string) => PartialParsed> = {
  javascript: parseJavaScript,
  python: parsePython,
  java: parseJava,
  go: parseGo,
}

function dedupeHeaders(headers: KeyValue[]): KeyValue[] {
  const out: KeyValue[] = []
  for (const h of headers) {
    const existing = out.find((x) => x.key.toLowerCase() === h.key.toLowerCase())
    if (existing) existing.value = h.value
    else out.push(h)
  }
  return out
}

/** 解析非 cURL 代码片段为 CurlParsed（cURL 请走后端 parse_curl_command）。 */
export function parseCodeSnippet(lang: SnippetLang, src: string): CurlParsed {
  if (lang === 'curl') throw new Error('cURL 请使用后端解析器（parseCurlCommand）')
  const partial = PARSERS[lang](src)
  const headers = dedupeHeaders(partial.headers)
  const ct = headers.find((h) => h.key.toLowerCase() === 'content-type')?.value ?? ''
  const raw = partial.bodyFromObject ?? partial.bodyRaw
  // 对象字面量（JS/Python dict）转 JSON 文本：键加引号（JSON5 宽松写法兼容）。
  let body: BodySpec | null = null
  if (raw !== null && raw.trim()) {
    body = inferBody(partial.bodyFromObject !== null ? normalizeObjectLiteral(raw) : raw, ct)
  }
  const auth: AuthSpec = { type: 'none' }
  return { url: partial.url, method: partial.method, headers, body, auth }
}

/** JS/Python 对象字面量 → 尽力规整为合法 JSON 文本（裸键加引号、单引号换双引号、去尾逗号）。 */
function normalizeObjectLiteral(src: string): string {
  const inner = src.trim()
  if (looksLikeJson(inner)) {
    try {
      return JSON.stringify(JSON.parse(inner))
    } catch {
      return inner
    }
  }
  let out = inner
  // 裸键加引号：{ name: → { "name":
  out = out.replace(/([{,]\s*)([A-Za-z_$][\w$-]*)\s*:/g, '$1"$2":')
  // 单引号字符串 → 双引号（内容中的双引号转义）
  out = out.replace(/'((?:[^'\\]|\\.)*)'/g, (_, body: string) => `"${body.replace(/"/g, '\\"')}"`)
  // 尾逗号
  out = out.replace(/,(\s*[}\]])/g, '$1')
  try {
    return JSON.stringify(JSON.parse(out))
  } catch {
    return inner
  }
}

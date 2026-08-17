/**
 * url.ts：URL 拆解工具（从 workspace store 抽出的纯函数）。
 */
import type { KeyValue } from '../types/foxApi'

/** 可选的协议前缀（地址栏协议选择器取值集合）。 */
export const PROTOCOLS = ['https', 'http', 'wss', 'ws'] as const
export type Protocol = (typeof PROTOCOLS)[number]

/** 匹配完整协议前缀：http / https / ws / wss。 */
const SCHEME_RE = /^(https?|wss?):\/\//i

/** 从域名源提取协议；无法识别时回退 https。 */
export function protocolFromDomain(src: string): Protocol {
  const m = SCHEME_RE.exec(src)
  const s = (m?.[1] ?? 'https').toLowerCase() as Protocol
  return PROTOCOLS.includes(s) ? s : 'https'
}

/** 去掉协议前缀，返回裸域名/主机部分。 */
export function stripProtocol(src: string): string {
  return src.replace(SCHEME_RE, '')
}

/** 替换（或补全）协议前缀：`api.x.com` → `https://api.x.com`。 */
export function withProtocol(src: string, protocol: Protocol): string {
  return `${protocol}://${stripProtocol(src)}`
}

/** 把导入 URL 拆成路径 + 查询参数 + origin；无 scheme 时按 https 补全。 */
export function splitUrl(
  url: string,
): {
  path: string
  params: KeyValue[]
  origin: string
} {
  let target = url.trim()
  if (!/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(target)) {
    target = `https://${target}`
  }
  const parsed = new URL(target)
  const params: KeyValue[] = []
  parsed.searchParams.forEach((value, key) => {
    params.push({ key, value, enabled: true, description: '' })
  })
  let path = parsed.pathname
  if (!path.startsWith('/')) path = `/${path}`
  return { path, params, origin: parsed.origin }
}

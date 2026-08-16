/**
 * url.ts：URL 拆解工具（从 workspace store 抽出的纯函数）。
 */
import type { KeyValue } from '../types/foxApi'

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

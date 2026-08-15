/**
 * environment.ts：环境相关的前端工具（颜色归类 / 变量解析）。
 *
 * - envColorClass：按环境名称启发式归类颜色（开发绿 / 测试蓝 / 预发布琥珀 / 生产橙 / 全局紫）；
 *   组件内用 `.ed-{class}` 映射到设计令牌（禁止写死色值）；
 * - resolveVariables：镜像 fox-core variable.rs 的 `{{name}}` 单层递归解析（深度上限 10），
 *   用于地址栏前缀 chip 的「解析后预览」；未知变量原样保留。
 */

import type { Environment } from '../types/foxApi'

/** 环境名称 → 颜色类（映射类名，颜色值由调用方 scoped CSS 定义）。 */
export function envColorClass(name: string): string {
  const n = name.trim().toLowerCase()
  if (/(开发|development|dev)/.test(n)) return 'dev'
  if (/(测试|test|qa)/.test(n)) return 'test'
  if (/(预发布|staging|stage|pre)/.test(n)) return 'staging'
  if (/(生产|prod|production|live)/.test(n)) return 'prod'
  if (/(全局|global)/.test(n)) return 'global'
  return ''
}

/** 环境的「主 baseUrl」：优先取 base_url 变量，无则回退为「无」。 */
export function envBaseUrl(env: Environment | null | undefined): string {
  const v = env?.variables?.base_url?.trim()
  return v || ''
}

/** 规范化基础 URL：去掉尾部斜杠，避免与路径拼接出双斜杠（`https://x.com//posts`）。 */
export function normalizeBaseUrl(value: string): string {
  const s = value.trim()
  const stripped = s.replace(/\/+$/, '')
  // 保留协议完整性（避免 `https://` 被削成 `https:`）
  if (/^[a-zA-Z][a-zA-Z0-9+.-]*:\/+$/.test(s)) return s
  return stripped
}

/** 变量递归解析（镜像后端：单次扫描 + 深度上限；未知变量原样保留）。 */
export function resolveVariables(
  input: string,
  vars: Record<string, string>,
  depth = 0,
): string {
  if (depth >= 10) return input
  return input.replace(/\{\{\s*([^{}]+?)\s*\}\}/g, (full, name: string) => {
    const value = vars[name]
    if (value == null || value === '') return full
    return resolveVariables(value, vars, depth + 1)
  })
}

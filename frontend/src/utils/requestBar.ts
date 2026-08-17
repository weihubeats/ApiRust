/**
 * requestBar.ts：请求栏（URL Bar）展示与 Method 智能默认的纯函数（可单测）。
 *
 * - envBadgeLabel / envBadgeTooltip：Base URL 标识文案（直接展示域名，便于
 *   一眼看出环境 base_url 是否解析、是否正确，避免「显示环境名」掩盖 DNS 问题）；
 * - applyMethodDefaults：切换 Method 时的智能默认（POST 系 → body 空则初始化
 *   JSON `{}` + Content-Type；其余 → params），返回应激活的 Tab。
 */
import type { RequestSpec } from '../types/foxApi'
import { applyRawSubtype } from './bodyMode'
import { stripProtocol } from './url'

export type SmartTab = 'body' | 'params'

/** 函数边界重新读取 body，打断 TS 对 `mode === 'none'` 的持久窄化（applyRawSubtype 会改写 body）。 */
function bodyOf(req: RequestSpec): RequestSpec['body'] {
  return req.body
}

/** POST / PUT / PATCH 系 → Body；其余 → Params。 */
export function methodNeedsBody(method: string): boolean {
  return method === 'POST' || method === 'PUT' || method === 'PATCH'
}

export function smartTabFor(method: string): SmartTab {
  return methodNeedsBody(method) ? 'body' : 'params'
}

/**
 * 应用 Method 智能默认：
 * - POST 系：body 为空（mode=none）时初始化为 JSON `{}` + Content-Type:
 *   application/json；已有内容保持原样。返回 'body'。
 * - 其余：不触碰 body。返回 'params'。
 */
export function applyMethodDefaults(req: RequestSpec, method: string): SmartTab {
  if (!methodNeedsBody(method)) return 'params'
  if (req.body.mode === 'none') {
    applyRawSubtype(req, 'json')
    const b = bodyOf(req)
    if (b.mode === 'json' && !b.raw.trim()) b.raw = '{}'
  }
  return 'body'
}

interface BadgeInput {
  /** 未解析的域名源（可能为 {{变量}} 或空）。 */
  urlDomain: string
  /** 变量解析后的实际域名（空 = 未配置/未解析）。 */
  resolvedDomain: string
  /** 激活环境名（无域名可显示时的兜底）。 */
  envName: string
}

/** Base URL 标识文案：优先展示解析后的裸域名（去除协议），一眼可见实际发送目标。 */
export function envBadgeLabel(input: BadgeInput): string {
  const src = input.resolvedDomain || input.urlDomain
  if (!src) return input.envName
  return stripProtocol(src) || src
}

/** Base URL 标识悬浮提示：`环境：X | 基础路径：https://...`。 */
export function envBadgeTooltip(input: BadgeInput): string {
  const base = input.resolvedDomain || input.urlDomain
  if (!base) return input.envName ? `环境：${input.envName}` : ''
  if (input.urlDomain.startsWith('{{') && !input.resolvedDomain) {
    return `${input.urlDomain} 未定义，请求将按字面量发送`
  }
  return input.envName
    ? `环境：${input.envName} | 基础路径：${input.resolvedDomain || input.urlDomain}`
    : `基础路径：${input.resolvedDomain || input.urlDomain}（会话 Base URL）`
}
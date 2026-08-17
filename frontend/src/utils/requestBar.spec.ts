import { describe, expect, it } from 'vitest'
import type { RequestSpec } from '../types/foxApi'
import { applyMethodDefaults, envBadgeLabel, envBadgeTooltip, methodNeedsBody, smartTabFor } from './requestBar'

function req(overrides: Partial<RequestSpec> = {}): RequestSpec {
  return {
    params: [],
    headers: [],
    path_variables: [],
    auth: { type: 'none' },
    body: { mode: 'none' },
    active_tab: null,
    timeout_ms: 30_000,
    follow_redirects: true,
    tests: null,
    ...overrides,
  }
}

describe('methodNeedsBody / smartTabFor', () => {
  it('POST / PUT / PATCH → Body；其余 → Params', () => {
    expect(methodNeedsBody('POST')).toBe(true)
    expect(methodNeedsBody('PUT')).toBe(true)
    expect(methodNeedsBody('PATCH')).toBe(true)
    expect(methodNeedsBody('GET')).toBe(false)
    expect(methodNeedsBody('DELETE')).toBe(false)
    expect(methodNeedsBody('HEAD')).toBe(false)
    expect(methodNeedsBody('OPTIONS')).toBe(false)
    expect(smartTabFor('POST')).toBe('body')
    expect(smartTabFor('GET')).toBe('params')
  })
})

describe('applyMethodDefaults', () => {
  it('POST + 空体 → 初始化为 JSON {} + Content-Type application/json，返回 body', () => {
    const r = req()
    const tab = applyMethodDefaults(r, 'POST')
    expect(tab).toBe('body')
    expect(r.body).toEqual({ mode: 'json', raw: '{}' })
    expect(r.headers).toContainEqual({
      key: 'Content-Type',
      value: 'application/json',
      enabled: true,
      description: '',
    })
  })

  it('POST + 已有 body 内容 → 保留原样，仍返回 body', () => {
    const r = req({ body: { mode: 'json', raw: '{"a":1}' } })
    const tab = applyMethodDefaults(r, 'POST')
    expect(tab).toBe('body')
    expect(r.body).toEqual({ mode: 'json', raw: '{"a":1}' })
  })

  it('POST + 非空 text body → 不强制 JSON，保留原样', () => {
    const r = req({ body: { mode: 'text', raw: 'hello' } })
    applyMethodDefaults(r, 'POST')
    expect(r.body).toEqual({ mode: 'text', raw: 'hello' })
  })

  it('GET / DELETE → 返回 params，绝不触碰 body', () => {
    const r = req()
    expect(applyMethodDefaults(r, 'GET')).toBe('params')
    expect(applyMethodDefaults(r, 'DELETE')).toBe('params')
    expect(r.body).toEqual({ mode: 'none' })
  })
})

describe('envBadgeLabel', () => {
  const base = { urlDomain: 'https://paymentv2test.redotpay.net', resolvedDomain: '', envName: 'Test' }

  it('有解析域名 → 直接展示裸域名（去协议）', () => {
    expect(
      envBadgeLabel({ ...base, resolvedDomain: 'https://paymentv2test.redotpay.net' }),
    ).toBe('paymentv2test.redotpay.net')
  })

  it('无域名可显示 → 退回环境名', () => {
    expect(envBadgeLabel({ urlDomain: '', resolvedDomain: '', envName: 'Test' })).toBe('Test')
  })

  it('变量未解析 → 展示字面量（含协议原文）', () => {
    expect(envBadgeLabel({ urlDomain: '{{base_url}}', resolvedDomain: '', envName: 'Test' })).toBe('{{base_url}}')
  })
})

describe('envBadgeTooltip', () => {
  it('环境已解析 → 环境：X | 基础路径：完整 URL', () => {
    expect(
      envBadgeTooltip({
        urlDomain: '{{base_url}}',
        resolvedDomain: 'https://paymentv2test.redotpay.net',
        envName: 'Test',
      }),
    ).toBe('环境：Test | 基础路径：https://paymentv2test.redotpay.net')
  })

  it('变量未定义 → 字面量警告（DNS 失败的直观提示）', () => {
    expect(
      envBadgeTooltip({ urlDomain: '{{base_url}}', resolvedDomain: '', envName: 'Test' }),
    ).toBe('{{base_url}} 未定义，请求将按字面量发送')
  })

  it('无环境（会话 Base URL）→ 标注会话来源', () => {
    expect(
      envBadgeTooltip({ urlDomain: 'https://api.x.com', resolvedDomain: 'https://api.x.com', envName: '' }),
    ).toBe('基础路径：https://api.x.com（会话 Base URL）')
  })

  it('无任何域名 → 仅展示环境名', () => {
    expect(envBadgeTooltip({ urlDomain: '', resolvedDomain: '', envName: 'Test' })).toBe('环境：Test')
    expect(envBadgeTooltip({ urlDomain: '', resolvedDomain: '', envName: '' })).toBe('')
  })
})
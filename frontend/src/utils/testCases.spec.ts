/**
 * testCases.ts 单测：请求快照提取 + 用例回填（各 Body 类型 / 容错降级）。
 */
import { describe, expect, it } from 'vitest'
import type { RequestSpec } from '../types/foxApi'
import {
  applyCaseToRequest,
  bodyContentOf,
  bodyTypeLabel,
  bodyTypeOf,
  formatDuration,
  restoreBody,
  snapshotRequest,
  statusTextOf,
  statusToneOf,
} from './testCases'

function req(overrides: Partial<RequestSpec> = {}): RequestSpec {
  return {
    params: [{ key: 'page', value: '1', enabled: true, description: '' }],
    headers: [{ key: 'X-Trace', value: 'on', enabled: true, description: '' }],
    path_variables: [],
    auth: { type: 'none' },
    body: { mode: 'none' },
    active_tab: null,
    timeout_ms: 30000,
    follow_redirects: true,
    tests: null,
    ...overrides,
  }
}

describe('formatDuration / statusTextOf / statusToneOf', () => {
  it('耗时：长浮点取整毫秒，≥1s 转秒，空值 -', () => {
    expect(formatDuration(206.91179200000002)).toBe('207ms')
    expect(formatDuration(145)).toBe('145ms')
    expect(formatDuration(1500)).toBe('1.50s')
    expect(formatDuration(60_000)).toBe('60.00s')
    expect(formatDuration(0)).toBe('0ms')
    expect(formatDuration(null)).toBe('-')
    expect(formatDuration(undefined)).toBe('-')
  })

  it('状态文案与色调：2xx 绿 / 4xx 琥珀 / 5xx 玫红 / 3xx 蓝', () => {
    expect(statusTextOf(200)).toBe('OK')
    expect(statusTextOf(500)).toBe('Internal Error')
    expect(statusTextOf(599)).toBe('599')
    expect(statusToneOf(200)).toBe('ok')
    expect(statusToneOf(404)).toBe('warn')
    expect(statusToneOf(500)).toBe('err')
    expect(statusToneOf(302)).toBe('info')
  })
})

describe('bodyTypeOf / bodyContentOf', () => {
  it('各 Body 模式映射到类型标识与文本快照', () => {
    expect(bodyTypeOf({ mode: 'json', raw: '{"a":1}' })).toBe('json')
    expect(bodyTypeOf({ mode: 'text', raw: 'hello' })).toBe('raw')
    expect(bodyTypeOf({ mode: 'urlencoded', fields: [] })).toBe('urlencoded')
    expect(bodyTypeOf({ mode: 'multipart', fields: [] })).toBe('form-data')
    expect(bodyTypeOf({ mode: 'graphql', spec: { query: 'q', variables: '', operation_name: '' } })).toBe('graphql')
    expect(bodyTypeOf({ mode: 'binary', path: '/tmp/a.png' })).toBe('binary')
    expect(bodyTypeOf({ mode: 'none' })).toBe('none')

    expect(bodyContentOf({ mode: 'json', raw: '{"a":1}' })).toBe('{"a":1}')
    expect(bodyContentOf({ mode: 'binary', path: '/tmp/a.png' })).toBe('/tmp/a.png')
    expect(bodyContentOf({ mode: 'none' })).toBe('')
  })

  it('bodyTypeLabel 有可读文案', () => {
    expect(bodyTypeLabel('json')).toBe('JSON')
    expect(bodyTypeLabel('form-data')).toBe('Form-Data')
    expect(bodyTypeLabel('unknown')).toBe('unknown')
  })
})

describe('snapshotRequest', () => {
  it('深拷贝 params / headers（后续修改互不影响）', () => {
    const r = req()
    const snap = snapshotRequest(r)
    r.params[0].value = '99'
    r.headers[0].key = 'mutated'
    expect(snap.params[0].value).toBe('1')
    expect(snap.headers[0].key).toBe('X-Trace')
  })
})

describe('restoreBody', () => {
  it('json 保留原文（可回填），raw → text，none → 无 Body', () => {
    expect(restoreBody('json', '{"a":1}')).toEqual({ mode: 'json', raw: '{"a":1}' })
    expect(restoreBody('raw', 'plain')).toEqual({ mode: 'text', raw: 'plain' })
    expect(restoreBody('none', '')).toEqual({ mode: 'none' })
  })

  it('urlencoded / form-data 还原字段', () => {
    const kv = [{ key: 'a', value: '1', enabled: true, description: '' }]
    expect(restoreBody('urlencoded', JSON.stringify(kv))).toEqual({ mode: 'urlencoded', fields: kv })
    expect(restoreBody('form-data', JSON.stringify(kv))).toEqual({
      mode: 'multipart',
      fields: [{ key: 'a', value: '1', value_type: 'text', enabled: true }],
    })
  })

  it('graphql / binary 还原', () => {
    expect(restoreBody('graphql', 'query { user }')).toEqual({
      mode: 'graphql',
      spec: { query: 'query { user }', variables: '', operation_name: '' },
    })
    expect(restoreBody('binary', '/tmp/a.png')).toEqual({ mode: 'binary', path: '/tmp/a.png' })
  })

  it('字段 JSON 损坏时降级为空数组，未知类型降级为 raw 文本', () => {
    expect(restoreBody('urlencoded', 'not-json')).toEqual({ mode: 'urlencoded', fields: [] })
    expect(restoreBody('unknown-type', 'x')).toEqual({ mode: 'text', raw: 'x' })
  })
})

describe('applyCaseToRequest', () => {
  it('整体替换 params / headers / body', () => {
    const r = req()
    applyCaseToRequest(r, {
      params: [{ key: 'debug', value: '0', enabled: true, description: '' }],
      headers: [],
      body_type: 'json',
      body_content: '{"amount":100}',
    })
    expect(r.params).toEqual([{ key: 'debug', value: '0', enabled: true, description: '' }])
    expect(r.headers).toEqual([])
    expect(r.body).toEqual({ mode: 'json', raw: '{"amount":100}' })
  })

  it('回填不污染原快照数据（深拷贝）', () => {
    const snap = {
      params: [{ key: 'a', value: '1', enabled: true, description: '' }],
      headers: [],
      body_type: 'json',
      body_content: '{}',
    }
    const r = req()
    applyCaseToRequest(r, snap)
    r.params[0].value = 'changed'
    expect(snap.params[0].value).toBe('1')
  })
})

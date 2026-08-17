/**
 * bodyMode 单测：Tab ↔ BodySpec 映射、raw 子类型推导与 Content-Type 联动。
 */
import { describe, expect, it } from 'vitest'
import type { BodySpec, RequestSpec } from '../types/foxApi'
import {
  RAW_SUBTYPES,
  applyBodyTab,
  applyRawSubtype,
  contentTypeOf,
  rawSubtypeOf,
  tabOf,
} from './bodyMode'

function spec(body: BodySpec, headers: RequestSpec['headers'] = []): RequestSpec {
  return {
    params: [],
    headers,
    path_variables: [],
    auth: { type: 'none' },
    body,
    timeout_ms: 30000,
    follow_redirects: true,
    tests: null,
  }
}

describe('tabOf', () => {
  it('BodySpec 映射到对应 Tab', () => {
    expect(tabOf({ mode: 'none' }, [])).toBe('none')
    expect(tabOf({ mode: 'multipart', fields: [] }, [])).toBe('form-data')
    expect(tabOf({ mode: 'urlencoded', fields: [] }, [])).toBe('x-www-form-urlencoded')
    expect(tabOf({ mode: 'binary', path: '/tmp/a' }, [])).toBe('binary')
    expect(tabOf({ mode: 'graphql', spec: { query: '', variables: '{}', operation_name: '' } }, [])).toBe('graphql')
    expect(tabOf({ mode: 'json', raw: '{}' }, [])).toBe('raw')
    expect(tabOf({ mode: 'text', raw: 'x' }, [])).toBe('raw')
  })
})

describe('rawSubtypeOf', () => {
  it('json 模式恒为 JSON', () => {
    expect(rawSubtypeOf({ mode: 'json', raw: '{}' }, [])).toBe('json')
  })

  it('text 模式按 Content-Type 头推导子类型', () => {
    const h = (v: string) => [{ key: 'Content-Type', value: v, enabled: true, description: '' }]
    expect(rawSubtypeOf({ mode: 'text', raw: '' }, h('text/html'))).toBe('html')
    expect(rawSubtypeOf({ mode: 'text', raw: '' }, h('application/xml'))).toBe('xml')
    expect(rawSubtypeOf({ mode: 'text', raw: '' }, h('text/javascript'))).toBe('javascript')
    expect(rawSubtypeOf({ mode: 'text', raw: '' }, h('text/plain'))).toBe('text')
    expect(rawSubtypeOf({ mode: 'text', raw: '' }, [])).toBe('text')
  })
})

describe('applyRawSubtype', () => {
  it('JSON 子类型 → json 模式 + application/json 头', () => {
    const req = spec({ mode: 'text', raw: '{"a":1}' })
    applyRawSubtype(req, 'json')
    expect(req.body).toEqual({ mode: 'json', raw: '{"a":1}' })
    expect(contentTypeOf(req.headers)).toBe('application/json')
  })

  it('XML 子类型 → text 模式 + application/xml 头，raw 文本保留', () => {
    const req = spec({ mode: 'json', raw: '<a/>' })
    applyRawSubtype(req, 'xml')
    expect(req.body).toEqual({ mode: 'text', raw: '<a/>' })
    expect(contentTypeOf(req.headers)).toBe('application/xml')
  })

  it('原位更新已有 Content-Type 行而不是追加', () => {
    const req = spec({ mode: 'text', raw: '' }, [
      { key: 'content-type', value: 'text/plain', enabled: true, description: '' },
      { key: 'X-Token', value: 't', enabled: true, description: '' },
    ])
    applyRawSubtype(req, 'html')
    expect(req.headers).toHaveLength(2)
    expect(req.headers[0]).toMatchObject({ key: 'Content-Type', value: 'text/html' })
  })
})

describe('applyBodyTab', () => {
  it('raw 聚合视图：进入 raw 时保留原 json/text 内容与子类型', () => {
    const req = spec({ mode: 'json', raw: '{"a":1}' })
    applyBodyTab(req, 'raw')
    expect(req.body).toEqual({ mode: 'json', raw: '{"a":1}' })
    expect(contentTypeOf(req.headers)).toBe('application/json')
  })

  it('form-data：urlencoded 字段转换为文本 part，且移除 Content-Type（boundary 由执行器生成）', () => {
    const req = spec(
      { mode: 'urlencoded', fields: [{ key: 'a', value: '1', enabled: true, description: '' }] },
      [{ key: 'Content-Type', value: 'application/json', enabled: true, description: '' }],
    )
    applyBodyTab(req, 'form-data')
    expect(req.body).toEqual({
      mode: 'multipart',
      fields: [{ key: 'a', value_type: 'text', value: '1', enabled: true }],
    })
    expect(contentTypeOf(req.headers)).toBe('')
  })

  it('x-www-form-urlencoded：multipart 文本 part 退化为 kv 行 + 固定 MIME', () => {
    const req = spec({
      mode: 'multipart',
      fields: [{ key: 'a', value_type: 'text', value: '1', enabled: true }],
    })
    applyBodyTab(req, 'x-www-form-urlencoded')
    expect(req.body).toEqual({
      mode: 'urlencoded',
      fields: [{ key: 'a', value: '1', enabled: true, description: '' }],
    })
    expect(contentTypeOf(req.headers)).toBe('application/x-www-form-urlencoded')
  })

  it('binary：固定 octet-stream 头', () => {
    const req = spec({ mode: 'none' })
    applyBodyTab(req, 'binary')
    expect(req.body).toEqual({ mode: 'binary', path: '' })
    expect(contentTypeOf(req.headers)).toBe('application/octet-stream')
  })

  it('none：移除 Content-Type', () => {
    const req = spec(
      { mode: 'none' },
      [{ key: 'Content-Type', value: 'application/json', enabled: true, description: '' }],
    )
    applyBodyTab(req, 'none')
    expect(contentTypeOf(req.headers)).toBe('')
  })

  it('graphql：进入时初始化空 spec + application/json 头', () => {
    const req = spec({ mode: 'none' })
    applyBodyTab(req, 'graphql')
    expect(req.body).toEqual({
      mode: 'graphql',
      spec: { query: '', variables: '{}', operation_name: '' },
    })
    expect(contentTypeOf(req.headers)).toBe('application/json')
  })
})

describe('RAW_SUBTYPES', () => {
  it('五个子类型均有唯一 MIME', () => {
    expect(RAW_SUBTYPES.map((s) => s.value)).toEqual(['json', 'text', 'javascript', 'html', 'xml'])
    expect(new Set(RAW_SUBTYPES.map((s) => s.mime)).size).toBe(RAW_SUBTYPES.length)
  })
})

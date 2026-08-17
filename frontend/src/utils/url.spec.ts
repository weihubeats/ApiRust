import { describe, expect, it } from 'vitest'
import { splitUrl, protocolFromDomain, stripProtocol, withProtocol } from './url'

describe('splitUrl', () => {
  it('完整 URL 拆出路径、查询参数与 origin', () => {
    const r = splitUrl('https://api.x.com/posts?userId=1&page=2')
    expect(r.path).toBe('/posts')
    expect(r.origin).toBe('https://api.x.com')
    expect(r.params).toEqual([
      { key: 'userId', value: '1', enabled: true, description: '' },
      { key: 'page', value: '2', enabled: true, description: '' },
    ])
  })

  it('无 scheme 时按 https 补全；无路径时补 /', () => {
    const r = splitUrl('api.x.com')
    expect(r.origin).toBe('https://api.x.com')
    expect(r.path).toBe('/')
    expect(r.params).toEqual([])
  })
})

describe('protocolFromDomain', () => {
  it('识别本地图标的四种协议', () => {
    expect(protocolFromDomain('https://api.x.com')).toBe('https')
    expect(protocolFromDomain('http://localhost:3000')).toBe('http')
    expect(protocolFromDomain('wss://s.x.com/ws')).toBe('wss')
    expect(protocolFromDomain('ws://s.x.com')).toBe('ws')
  })

  it('无 scheme / 变量引用 / 无法识别时回退 https', () => {
    expect(protocolFromDomain('api.x.com')).toBe('https')
    expect(protocolFromDomain('{{base_url}}')).toBe('https')
    expect(protocolFromDomain('')).toBe('https')
  })

  it('大小写不敏感', () => {
    expect(protocolFromDomain('HTTPS://api.x.com')).toBe('https')
    expect(protocolFromDomain('Wss://s.x.com')).toBe('wss')
  })
})

describe('stripProtocol / withProtocol', () => {
  it('stripProtocol 去掉协议前缀，保留其余部分', () => {
    expect(stripProtocol('https://api.x.com/path')).toBe('api.x.com/path')
    expect(stripProtocol('localhost:3000')).toBe('localhost:3000')
    expect(stripProtocol('{{base_url}}')).toBe('{{base_url}}')
  })

  it('withProtocol 替换或补全协议', () => {
    expect(withProtocol('http://api.x.com', 'https')).toBe('https://api.x.com')
    expect(withProtocol('api.x.com', 'https')).toBe('https://api.x.com')
    expect(withProtocol('ws://s.x.com', 'wss')).toBe('wss://s.x.com')
  })
})

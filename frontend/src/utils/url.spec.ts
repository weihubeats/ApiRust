import { describe, expect, it } from 'vitest'
import { splitUrl } from './url'

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

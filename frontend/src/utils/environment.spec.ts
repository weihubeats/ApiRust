import { describe, expect, it } from 'vitest'
import {
  envBaseUrl,
  envColorClass,
  normalizeBaseUrl,
  resolveVariables,
} from './environment'

describe('envColorClass', () => {
  it('按名称启发式归类（中英文大小写不敏感）', () => {
    expect(envColorClass('开发环境')).toBe('dev')
    expect(envColorClass('Development')).toBe('dev')
    expect(envColorClass('QA')).toBe('test')
    expect(envColorClass('Staging')).toBe('staging')
    expect(envColorClass('production')).toBe('prod')
    expect(envColorClass('全局')).toBe('global')
    expect(envColorClass('自定义')).toBe('')
  })
})

describe('envBaseUrl', () => {
  it('取 base_url 变量，空环境返回空串', () => {
    expect(
      envBaseUrl({ variables: { base_url: ' https://x.com/ ' } } as never),
    ).toBe('https://x.com/')
    expect(envBaseUrl(null)).toBe('')
  })
})

describe('normalizeBaseUrl', () => {
  it('去掉尾部斜杠但保留协议本身', () => {
    expect(normalizeBaseUrl('https://x.com/')).toBe('https://x.com')
    expect(normalizeBaseUrl('https://x.com///')).toBe('https://x.com')
    expect(normalizeBaseUrl('https://')).toBe('https://')
  })
})

describe('resolveVariables', () => {
  const vars = { base: '{{host}}/api', host: 'https://x.com', empty: '' }

  it('递归解析已知变量', () => {
    expect(resolveVariables('{{base}}/posts', vars)).toBe(
      'https://x.com/api/posts',
    )
  })

  it('未知或空值变量原样保留', () => {
    expect(resolveVariables('{{nope}}', vars)).toBe('{{nope}}')
    expect(resolveVariables('{{empty}}', vars)).toBe('{{empty}}')
  })

  it('循环引用在深度上限处停止', () => {
    const cyclic = { a: '{{b}}', b: '{{a}}' }
    const out = resolveVariables('{{a}}', cyclic)
    expect(out).toContain('{{')
  })
})

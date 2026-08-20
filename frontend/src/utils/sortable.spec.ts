import { afterEach, describe, expect, it, vi } from 'vitest'

const { mount } = vi.hoisted(() => ({ mount: vi.fn() }))

vi.mock('sortablejs', () => ({
  default: { mount },
  Swap: class Swap {},
}))

import { ensureSwapMounted } from './sortable'

afterEach(() => {
  vi.clearAllMocks()
})

describe('ensureSwapMounted', () => {
  it('重复挂载时静默吞掉 "Cannot mount plugin" 错误（HMR 场景）', () => {
    mount.mockImplementation(() => {
      throw new Error('Sortable: Cannot mount plugin Swap more than once')
    })
    expect(() => {
      ensureSwapMounted()
      ensureSwapMounted()
    }).not.toThrow()
    expect(mount).toHaveBeenCalledTimes(2)
  })

  it('非重复挂载错误照常抛出', () => {
    mount.mockImplementation(() => {
      throw new Error('other error')
    })
    expect(() => ensureSwapMounted()).toThrow('other error')
  })
})
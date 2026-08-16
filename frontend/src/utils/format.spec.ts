import { describe, expect, it } from 'vitest'
import { formatBytes, formatDuration } from './format'

describe('formatDuration', () => {
  it('空值与 NaN 显示占位符', () => {
    expect(formatDuration(null)).toBe('-')
    expect(formatDuration(undefined)).toBe('-')
    expect(formatDuration(Number.NaN)).toBe('-')
  })

  it('按量级自动选择单位', () => {
    expect(formatDuration(1744)).toBe('1.74 s')
    expect(formatDuration(2000)).toBe('2.00 s')
    expect(formatDuration(0.04)).toBe('40 µs')
    expect(formatDuration(0)).toBe('<1 ms')
  })
})

describe('formatBytes', () => {
  it('B / KB / MB 自动单位', () => {
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(2048)).toBe('2.0 KB')
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.00 MB')
  })
})

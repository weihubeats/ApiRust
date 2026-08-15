/**
 * format.ts：通用格式化工具。
 */

/**
 * 耗时格式化：自动选择单位（亚毫秒 → µs，毫秒 → ms，秒 → s）。
 * 例：1744 (µs) → "1.74 ms"；2000 → "2.00 s"；0.04 → "40 µs"。
 */
export function formatDuration(ms: number | null | undefined): string {
  if (ms == null || Number.isNaN(ms)) return '-'
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)} s`
  if (ms >= 1) return `${ms.toFixed(2)} ms`
  if (ms > 0) return `${Math.round(ms * 1000)} µs`
  return '<1 ms'
}

/** 字节数：B / KB / MB 自动单位。 */
export function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`
  return `${(n / (1024 * 1024)).toFixed(2)} MB`
}
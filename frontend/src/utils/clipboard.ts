/**
 * clipboard.ts：剪贴板写入工具（多级降级）。
 *
 * 尝试顺序：
 * 1. `navigator.clipboard.writeText`（异步 Clipboard API，要求安全上下文 + 权限）；
 * 2. 原生系统剪贴板写命令（Tauri：`plugin:fox|clipboard_write_text`，arboard 直写，
 *    规避 WKWebView 等 WebView 的剪贴板限制）；
 * 3. 隐藏 textarea + `document.execCommand('copy')`（旧 WebView / 受限 iframe）。
 *
 * 全部失败返回 false，由调用方决定 UI 兜底（如打开代码预览弹窗手动复制）。
 * 降级复制后会还原用户原选区，避免打断编辑器焦点。
 */

/** Tauri v2 注入的全局（存在即说明运行在 Tauri WebView 中）。 */
function inTauriWebview(): boolean {
  return (
    typeof window !== 'undefined' &&
    !!(window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__
  )
}

/** 惰性获取 Tauri invoke（非 Tauri 环境 / 加载失败返回 null）。 */
let tauriInvoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null | undefined

async function getTauriInvoke(): Promise<
  ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null
> {
  if (tauriInvoke !== undefined) return tauriInvoke
  try {
    const core = await import('@tauri-apps/api/core')
    tauriInvoke = core.invoke
  } catch {
    tauriInvoke = null
  }
  return tauriInvoke
}

export async function copyText(text: string): Promise<boolean> {
  // Tauri WebView：优先原生系统剪贴板命令（WKWebView 的 Clipboard API 无
  // 用户手势即拒，execCommand 会触发 IMK 噪音且常失效），最可靠路径放最前。
  if (typeof window !== 'undefined' && inTauriWebview()) {
    try {
      const invoke = await getTauriInvoke()
      await invoke?.('plugin:fox|clipboard_write_text', { text })
      return true
    } catch (err) {
      console.error('[clipboard] 原生剪贴板命令失败（请确认已重新编译应用）', err)
    }
  }

  // 1) 异步 Clipboard API。
  if (typeof navigator !== 'undefined' && navigator.clipboard?.writeText) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch (err) {
      console.error('[clipboard] Clipboard API 失败', err)
    }
  }

  // 2) 旧式 execCommand 降级。
  if (typeof document === 'undefined') return false

  const ta = document.createElement('textarea')
  ta.value = text
  ta.setAttribute('readonly', '')
  ta.style.position = 'fixed'
  ta.style.top = '-9999px'
  ta.style.opacity = '0'
  document.body.appendChild(ta)

  const selection = document.getSelection()
  const prevRange = selection && selection.rangeCount > 0 ? selection.getRangeAt(0) : null
  ta.select()

  let ok = false
  try {
    ok = document.execCommand('copy')
  } catch {
    ok = false
  } finally {
    document.body.removeChild(ta)
    if (prevRange && selection) {
      selection.removeAllRanges()
      selection.addRange(prevRange)
    }
  }
  return ok
}
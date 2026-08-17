/**
 * testUtils：组件测试通用工具。
 * - collectErrors：收集 window error / unhandledrejection / console.error，
 *   用于断言交互（含卸载间隙的异步回调）未产生未处理异常；
 * - stubScrollIntoView：jsdom 未实现 scrollIntoView，统一打桩以便计数。
 */
export interface ErrorCollector {
  /** 收集到的错误（断言应为空以保稳定）。 */
  errors: unknown[]
  /** 在 afterEach 中调用，恢复原始捕获。 */
  restore: () => void
}

export function collectErrors(): ErrorCollector {
  const errors: unknown[] = []
  const onError = (e: ErrorEvent): void => {
    errors.push(e.error ?? e.message)
  }
  const onRejection = (e: PromiseRejectionEvent): void => {
    errors.push(e.reason)
  }
  const onConsole = (...args: unknown[]): void => {
    errors.push(args)
  }
  window.addEventListener('error', onError)
  window.addEventListener('unhandledrejection', onRejection)
  const original = console.error
  console.error = onConsole
  return {
    errors,
    restore: () => {
      window.removeEventListener('error', onError)
      window.removeEventListener('unhandledrejection', onRejection)
      console.error = original
    },
  }
}

export function stubScrollIntoView(): () => void {
  const original = HTMLElement.prototype.scrollIntoView
  HTMLElement.prototype.scrollIntoView = (() => {}) as typeof original
  return () => {
    HTMLElement.prototype.scrollIntoView = original
  }
}
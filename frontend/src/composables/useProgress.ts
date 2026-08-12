/**
 * useProgress：全局顶部加载进度条（NProgress 风格）。
 *
 * 模块级单例：任何组件调用都是同一进度条。挂载 <ProgressBar /> 后，
 * `start()` 显示并递增到 ~90%，`done()` 拉满后淡出。
 *
 * 用法（useFoxApi 内部已自动接线，无需手动调用）：
 * ```ts
 * const progress = useProgress()
 * progress.start()   // 请求开始
 * progress.done()    // 请求结束
 * ```
 */
import { readonly, ref } from 'vue'

const visible = ref(false)
const progress = ref(0)
let timer: number | null = null

/** 简易流水动画：每 200ms 递增，越接近 90% 越慢。 */
function tick(): void {
  const remaining = 90 - progress.value
  progress.value += Math.max(0.5, remaining * 0.12)
  if (progress.value >= 90) {
    progress.value = 90
    if (timer !== null) {
      window.clearInterval(timer)
      timer = null
    }
  }
}

export function useProgress() {
  return {
    visible: readonly(visible),
    progress: readonly(progress),
    start(): void {
      visible.value = true
      progress.value = 8
      if (timer !== null) window.clearInterval(timer)
      timer = window.setInterval(tick, 200)
    },
    done(): void {
      if (timer !== null) {
        window.clearInterval(timer)
        timer = null
      }
      progress.value = 100
      window.setTimeout(() => {
        visible.value = false
        progress.value = 0
      }, 240)
    },
  }
}

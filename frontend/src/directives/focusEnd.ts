/**
 * focusEnd.ts：聚焦并把光标放到文本末尾 `v-focus-end`。
 *
 * - 适用于「重命名/编辑」预填输入框：autofocus 默认把光标放开头，
 *   用户想改后缀必须手动移动光标，体验割裂；
 * - mount 时立即聚焦 + 光标置尾，并持续监听 focus 兜底
 *   （Modal 的自动聚焦可能在指令挂载后才触发，会把光标重置回开头）；
 * - 空值输入框（新建）同样安全，len=0 无副作用。
 */
import type { Directive } from 'vue'

function placeEnd(el: HTMLInputElement | HTMLTextAreaElement): void {
  const len = el.value.length
  try {
    el.setSelectionRange(len, len)
  } catch {
    /* 非文本框（textarea 等无 selection API）忽略 */
  }
}

function onFocus(this: HTMLInputElement | HTMLTextAreaElement): void {
  placeEnd(this)
}

const focusEnd: Directive<HTMLInputElement | HTMLTextAreaElement> = {
  mounted(el) {
    el.focus()
    placeEnd(el)
    el.addEventListener('focus', onFocus)
  },
  unmounted(el) {
    el.removeEventListener('focus', onFocus)
  },
}

export default focusEnd
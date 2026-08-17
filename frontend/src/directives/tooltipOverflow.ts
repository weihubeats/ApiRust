/**
 * tooltipOverflow.ts：截断文本智能 Tooltip 指令 `v-tooltip-overflow="text"`。
 *
 * - 仅当文本真实溢出（el.scrollWidth > el.clientWidth）才显示，完整展示不打扰；
 * - 气泡相对触发元素定位（默认正下方居中，offset 6px，底部空间不足自动翻到上方）；
 * - 暗黑样式（.rf-overflow-tip，定义于 style.css）：半透明毛玻璃 + 描边 + 微阴影；
 * - 渐显过渡 + scroll/resize 实时重定位；鼠标离开 / 元素移除即销毁。
 */
import type { Directive, DirectiveBinding } from 'vue'

const OFFSET = 6
const DELAY_MS = 120

interface OverflowHost extends HTMLElement {
  __overflowContent?: string
}

let tip: HTMLDivElement | null = null
let host: OverflowHost | null = null
let showTimer: number | null = null

function position(): void {
  if (!tip || !host) return
  const rect = host.getBoundingClientRect()
  const tw = tip.offsetWidth
  const th = tip.offsetHeight
  let left = rect.left + rect.width / 2 - tw / 2
  let top = rect.bottom + OFFSET
  if (top + th > window.innerHeight - OFFSET && rect.top - th - OFFSET > OFFSET) {
    top = rect.top - th - OFFSET
  }
  left = Math.max(OFFSET, Math.min(left, window.innerWidth - tw - OFFSET))
  tip.style.left = `${left}px`
  tip.style.top = `${Math.max(OFFSET, top)}px`
}

function showTip(): void {
  showTimer = null
  if (!host) return
  tip = document.createElement('div')
  tip.className = 'rf-overflow-tip'
  tip.textContent = host.__overflowContent ?? ''
  document.body.appendChild(tip)
  position()
  requestAnimationFrame(() => tip?.classList.add('in'))
  window.addEventListener('scroll', onViewportChange, true)
  window.addEventListener('resize', onViewportChange)
}

function hide(): void {
  if (showTimer !== null) {
    window.clearTimeout(showTimer)
    showTimer = null
  }
  if (tip) {
    tip.remove()
    tip = null
  }
  host = null
  window.removeEventListener('scroll', onViewportChange, true)
  window.removeEventListener('resize', onViewportChange)
}

function onViewportChange(): void {
  if (!host || !host.isConnected) {
    hide()
    return
  }
  position()
}

function onEnter(event: MouseEvent): void {
  const el = event.currentTarget as OverflowHost
  const content = el.__overflowContent ?? ''
  if (!content) return
  // 智能溢出检测：未截断（能完整展示）则不弹，避免频繁闪烁。
  if (el.scrollWidth <= el.clientWidth + 1) return
  host = el
  showTimer = window.setTimeout(showTip, DELAY_MS)
}

function onLeave(): void {
  hide()
}

function onFocusIn(event: FocusEvent): void {
  onEnter(event as unknown as MouseEvent)
}

function mount(el: OverflowHost, binding: DirectiveBinding<string>): void {
  el.__overflowContent = binding.value
  el.addEventListener('mouseenter', onEnter)
  el.addEventListener('mouseleave', onLeave)
  el.addEventListener('focusin', onFocusIn)
  el.addEventListener('focusout', onLeave)
}

function unmount(el: OverflowHost): void {
  if (host === el) hide()
  el.removeEventListener('mouseenter', onEnter)
  el.removeEventListener('mouseleave', onLeave)
  el.removeEventListener('focusin', onFocusIn)
  el.removeEventListener('focusout', onLeave)
  delete el.__overflowContent
}

const tooltipOverflow: Directive<OverflowHost, string> = {
  mounted: mount,
  updated: (el, binding) => {
    if (binding.value !== binding.oldValue) el.__overflowContent = binding.value
  },
  unmounted: unmount,
}

export default tooltipOverflow
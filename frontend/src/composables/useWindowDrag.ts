/**
 * useWindowDrag：无边框/自定义标题栏窗口拖拽。
 * 绑定容器 mousedown：空白处（非交互元素）左键 → startDragging；双击 → 切换最大化。
 * 用法：给标题栏元素 ref，`useWindowDrag(topBarRef)`。
 */
import { onBeforeUnmount, watch, type Ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'

const appWindow = getCurrentWindow()

function onMouseDown(event: MouseEvent): void {
  if (event.button !== 0) return
  const target = event.target as HTMLElement
  if (target.closest('button, input, select, textarea, a, [contenteditable="true"]')) return
  if (event.detail === 2) {
    void appWindow.toggleMaximize()
  } else {
    void appWindow.startDragging()
  }
}

export function useWindowDrag(container: Ref<HTMLElement | null>): void {
  watch(container, (el, old) => {
    old?.removeEventListener('mousedown', onMouseDown)
    el?.addEventListener('mousedown', onMouseDown)
  })
  onBeforeUnmount(() => {
    container.value?.removeEventListener('mousedown', onMouseDown)
  })
}
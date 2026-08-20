import Sortable, { Swap } from 'sortablejs'

/** Swap 插件幂等挂载：HMR 重载模块会重复执行 mount，SortableJS 对同名插件二次挂载会抛错。 */
export function ensureSwapMounted(): void {
  try {
    Sortable.mount(new Swap())
  } catch (e) {
    if (e instanceof Error && e.message.includes('Cannot mount plugin')) return
    throw e
  }
}
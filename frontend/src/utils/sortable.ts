import Sortable, { Swap } from 'sortablejs'

/** 模块标记键：HMR 重灌模块时 top-level 会再次执行，
 *  但 import 进来的 Sortable 对象是同一份实例（不在 HMR 里重建），
 *  所以用它作挂载标记的载体，能跨 HMR 稳定判断。 */
const MARKER = '__rustfox_swap_mounted'

type AnySortable = typeof Sortable & { [MARKER]?: boolean }

/** Swap 插件幂等挂载：防止 "Cannot mount plugin Swap more than once"。
 *  HMR 重载 + 模块顶执行路径重复时，SortableJS 对同名插件二次 mount 会抛错。 */
export function ensureSwapMounted(): void {
  const s = Sortable as AnySortable
  if (s[MARKER]) return
  Sortable.mount(new Swap())
  s[MARKER] = true
}
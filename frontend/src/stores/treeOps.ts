/**
 * treeOps.ts：目录树拖拽重排的纯函数（从 workspace store 抽出以便单测）。
 *
 * 语义与原内联实现一致：
 * - 同组内移动：目标位超出边界时钳制到组尾；
 * - 跨组移动：旧组剩余项从 0 连续重排，新组按插入位重排；
 * - 返回 Map<id, 新 sort_order>（只包含发生变化的项），由调用方落库。
 */
export interface Orderable {
  id: string
  sort_order: number
}

/** 对组内其余项按插入位重排：插入位之前的保持 i，之后的顺延 +1。 */
function renumberAround<T extends Orderable>(
  group: T[],
  insertedIndex: number,
  skipId: string,
  changed: Map<string, number>,
): void {
  group
    .filter((f) => f.id !== skipId)
    .forEach((f, i) => {
      const order = i < insertedIndex ? i : i + 1
      if (f.sort_order !== order) changed.set(f.id, order)
    })
}

/** 同组内把 movedId 移到 targetIndex：返回所有需要落库的新顺序（含被移项）。 */
export function planSameGroupMove<T extends Orderable>(
  group: T[],
  movedId: string,
  targetIndex: number,
): Map<string, number> {
  const changed = new Map<string, number>()
  const clamped = Math.min(targetIndex, group.length - 1)
  renumberAround(group, clamped, movedId, changed)
  changed.set(movedId, clamped)
  return changed
}

/** 跨组移动：oldGroup 剩余项从 0 连续重排，newGroup（不含被移项）按插入位重排。 */
export function planCrossGroupMove<T extends Orderable>(
  oldGroup: T[],
  newGroup: T[],
  movedId: string,
  targetIndex: number,
): Map<string, number> {
  const changed = new Map<string, number>()
  renumberAround(oldGroup, 0, movedId, changed)
  const clamped = Math.min(targetIndex, newGroup.length)
  renumberAround(newGroup, clamped, movedId, changed)
  changed.set(movedId, clamped)
  return changed
}

/** 防环检查：newParentId 是否位于 folderId 自身或其子孙链上。 */
export function wouldCreateCycle(
  folders: Array<{ id: string; parent_id: string | null }>,
  folderId: string,
  newParentId: string | null,
): boolean {
  let cursor: string | null = newParentId
  while (cursor !== null) {
    if (cursor === folderId) return true
    cursor = folders.find((f) => f.id === cursor)?.parent_id ?? null
  }
  return false
}

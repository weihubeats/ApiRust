<script setup lang="ts">
/**
 * Menu：通用下拉菜单（Teleport 到 body，固定定位）。
 * - openAt(el, side) 依触发元素定位，底部空间不足自动上翻；
 * - 项支持 icon / danger（红色）/ disabled / dividerBefore 分隔线；
 * - 项带 confirm 文案时先进入行内确认视图，确认后 emit('confirm')；
 * - 外部点击 / Esc / 滚动 / 窗口缩放自动关闭。
 */
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import Icon from './Icon.vue'
import type { IconName } from './Icon.vue'

export interface MenuItem {
  key: string
  label: string
  icon?: IconName
  /** 选中态：右侧显示 check 图标并整行高亮。 */
  checked?: boolean
  /** 图标使用主题色（如「新建项目」的 +）。 */
  iconAccent?: boolean
  /** 右侧快捷键提示（如 ⌘N）。 */
  shortcut?: string
  danger?: boolean
  disabled?: boolean
  dividerBefore?: boolean
  confirm?: string
}

const emit = defineEmits<{
  select: [item: MenuItem]
  confirm: [item: MenuItem]
}>()

type View = { kind: 'list' } | { kind: 'confirm'; item: MenuItem }

const open = ref(false)
const view = ref<View>({ kind: 'list' })
const items = ref<MenuItem[]>([])
const pos = ref({ left: 0, top: 0 })
const menuEl = ref<HTMLElement | null>(null)

const menuStyle = computed(() => ({ left: `${pos.value.left}px`, top: `${pos.value.top}px` }))

function openAt(el: HTMLElement, menuItems: MenuItem[], side: 'right' | 'left' = 'right'): void {
  items.value = menuItems
  const rect = el.getBoundingClientRect()
  const width = 176
  const height = 220
  let left = side === 'right' ? rect.right - width : rect.left
  left = Math.max(8, Math.min(left, window.innerWidth - width - 8))
  let top = rect.bottom + 4
  if (top + height > window.innerHeight - 8 && rect.top - height - 4 > 8) {
    top = rect.top - height - 4
  }
  pos.value = { left, top }
  view.value = { kind: 'list' }
  open.value = true
}

function close(): void {
  open.value = false
  view.value = { kind: 'list' }
}

function onItemClick(item: MenuItem): void {
  if (item.disabled) return
  if (item.confirm) {
    view.value = { kind: 'confirm', item }
    return
  }
  close()
  emit('select', item)
}

function backToList(): void {
  view.value = { kind: 'list' }
}

function onConfirm(): void {
  const item = view.value.kind === 'confirm' ? view.value.item : null
  close()
  if (item) emit('confirm', item)
}

function onDocMouseDown(event: MouseEvent): void {
  const target = event.target as Node
  if (menuEl.value?.contains(target)) return
  close()
}

function onDocKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') close()
}

watch(open, (isOpen) => {
  if (isOpen) {
    document.addEventListener('mousedown', onDocMouseDown, true)
    document.addEventListener('keydown', onDocKeydown)
    document.addEventListener('scroll', close, true)
    window.addEventListener('resize', close)
  } else {
    document.removeEventListener('mousedown', onDocMouseDown, true)
    document.removeEventListener('keydown', onDocKeydown)
    document.removeEventListener('scroll', close, true)
    window.removeEventListener('resize', close)
  }
})

onBeforeUnmount(() => {
  document.removeEventListener('mousedown', onDocMouseDown, true)
  document.removeEventListener('keydown', onDocKeydown)
  document.removeEventListener('scroll', close, true)
  window.removeEventListener('resize', close)
})

defineExpose({ openAt, close })
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      ref="menuEl"
      class="rf-menu"
      :style="menuStyle"
      role="menu"
      @click.stop
    >
      <template v-if="view.kind === 'list'">
        <template v-for="item in items" :key="item.key">
          <div v-if="item.dividerBefore" class="rf-menu-divider"></div>
          <button
            class="rf-menu-item"
            :class="{ danger: item.danger, disabled: item.disabled, checked: item.checked }"
            type="button"
            role="menuitem"
            :disabled="item.disabled"
            @click="onItemClick(item)"
          >
            <span
              v-if="item.icon"
              class="rf-menu-icon"
              :class="{ accent: item.iconAccent }"
            >
              <Icon :name="item.icon" :size="14" />
            </span>
            <span class="rf-menu-label">{{ item.label }}</span>
            <kbd v-if="item.shortcut" class="rf-menu-kbd">{{ item.shortcut }}</kbd>
            <Icon
              v-else-if="item.checked"
              class="rf-menu-check"
              name="check"
              :size="13"
            />
          </button>
        </template>
      </template>
      <template v-else>
        <p class="rf-menu-confirm-title">{{ view.item.confirm }}</p>
        <div class="rf-menu-confirm-actions">
          <button class="rf-btn rf-btn-sm" type="button" @click="backToList">取消</button>
          <button class="rf-btn rf-btn-sm rf-btn-danger" type="button" @click="onConfirm">
            删除
          </button>
        </div>
      </template>
    </div>
  </Teleport>
</template>

<style scoped>
.rf-menu {
  position: fixed;
  z-index: 300;
  min-width: 176px;
  max-width: 240px;
  padding: 5px;
  background: var(--bg-elevated);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  box-shadow:
    var(--shadow-lg),
    0 20px 25px -5px rgb(0 0 0 / 0.3),
    0 8px 10px -6px rgb(0 0 0 / 0.3);
  animation: menu-in 120ms var(--ease);
  transform-origin: top center;
}

.rf-menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 7px 16px;
  border: none;
  background: none;
  border-radius: var(--radius-sm);
  font-size: 12.5px;
  font-family: inherit;
  color: var(--text-1);
  cursor: pointer;
  text-align: left;
  transition: background var(--dur) var(--ease), color var(--dur) var(--ease);
}
.rf-menu-item:hover {
  background: var(--bg-hover);
}
.rf-menu-item.checked {
  background: color-mix(in srgb, var(--accent) 14%, transparent);
  color: var(--text-1);
}
.rf-menu-item.danger {
  color: var(--danger);
}
.rf-menu-item.danger:hover {
  background: var(--danger-tint);
}
.rf-menu-item.disabled {
  color: var(--text-3);
  cursor: default;
}
.rf-menu-item.disabled:hover {
  background: none;
}

.rf-menu-icon {
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  color: var(--text-2);
}
.rf-menu-icon.accent {
  color: var(--accent);
}
.rf-menu-icon :deep(svg) {
  display: block;
}

.rf-menu-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.rf-menu-check {
  flex-shrink: 0;
  color: var(--accent);
}

.rf-menu-kbd {
  flex-shrink: 0;
  margin-left: 8px;
  padding: 1px 5px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--bg-hover);
  font-family: var(--font-mono);
  font-size: 10.5px;
  line-height: 1.3;
  color: var(--text-3);
}

.rf-menu-divider {
  height: 1px;
  margin: 6px 10px;
  background: var(--border-strong);
}

.rf-menu-confirm-title {
  margin: 2px 8px 12px;
  font-size: 12.5px;
  color: var(--text-1);
  word-break: break-all;
}

.rf-menu-confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  padding: 0 4px 2px;
}

@keyframes menu-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}
</style>
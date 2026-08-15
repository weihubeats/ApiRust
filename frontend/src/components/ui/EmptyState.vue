<script setup lang="ts">
/**
 * EmptyState：空态（图标 + 标题 + 描述 + 操作插槽）。
 */
import Icon from './Icon.vue'
import type { IconName } from './Icon.vue'

withDefaults(
  defineProps<{
    icon?: IconName
    title?: string
    description?: string
    compact?: boolean
  }>(),
  { icon: 'folder-open', title: '暂无数据', description: '', compact: false },
)
</script>

<template>
  <div class="es" :class="{ compact }">
    <span class="es-icon">
      <Icon :name="icon" :size="compact ? 20 : 26" :stroke-width="1.5" />
    </span>
    <p class="es-title">{{ title }}</p>
    <p v-if="description" class="es-desc">{{ description }}</p>
    <div v-if="$slots.default" class="es-actions">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.es {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px 16px;
  text-align: center;
}
.es.compact {
  padding: 24px 12px;
  gap: 6px;
}

.es-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 52px;
  height: 52px;
  border-radius: 16px;
  background: var(--bg-hover);
  color: var(--text-3);
}
.es.compact .es-icon {
  width: 40px;
  height: 40px;
  border-radius: 12px;
}

.es-title {
  margin: 0;
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text-1);
}

.es-desc {
  margin: 0;
  font-size: 12px;
  color: var(--text-3);
  max-width: 320px;
}

.es-actions {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}
</style>
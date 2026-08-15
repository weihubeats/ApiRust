<script setup lang="ts">
/**
 * ToolsDrawer：接口工具抽屉（压测 / 并发基准），点击请求栏「工具」按钮打开。
 * 生成代码与测试断言已迁至请求 Tab 的 Code / Tests 标签页，此处仅保留压测。
 */
import Drawer from './ui/Drawer.vue'
import LoadTestPanel from './LoadTestPanel.vue'
import type { Endpoint } from '../types/foxApi'

defineProps<{
  open: boolean
  draft: Endpoint | null
  url: string
}>()

const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <Drawer :open="open" title="压测（并发基准）" width="520px" @close="emit('close')">
    <LoadTestPanel :draft="draft" :url="url" />
  </Drawer>
</template>

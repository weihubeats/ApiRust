<script setup lang="ts">
/**
 * CodeExportDialog：接口代码导出弹窗。
 * 复用 CodePanel（语言选择 → 生成 → 复制），打开即按当前请求配置自动生成
 * curl / Python / JavaScript / Go / Java / PHP 代码片段。
 */
import CodePanel from './CodePanel.vue'
import Modal from './ui/Modal.vue'
import type { Endpoint } from '../types/foxApi'

defineProps<{ draft: Endpoint | null; url: string }>()
const emit = defineEmits<{ close: [] }>()
</script>

<template>
  <Modal :open="true" title="导出接口代码" width="680px" @close="emit('close')">
    <p class="modal-hint">
      按当前请求配置生成对应语言的代码片段，可直接粘贴到项目中使用。
    </p>
    <CodePanel :draft="draft" :url="url" auto-generate />
    <template #footer>
      <button class="rf-btn rf-btn-primary" type="button" @click="emit('close')">关闭</button>
    </template>
  </Modal>
</template>

<style scoped>
.modal-hint {
  margin: 0 0 10px;
  font-size: 12.5px;
  color: var(--text-2);
}
</style>
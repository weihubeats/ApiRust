<script setup lang="ts">
/**
 * ProjectDeleteModal：删除项目确认弹窗。
 *
 * - project 为 null 时关闭；
 * - 确认后调用后端删除并 emit('deleted', id)，由父级移出列表。
 */
import Modal from '../ui/Modal.vue'
import { useFoxApi } from '../../composables/useFoxApi'
import { useToast } from '../../composables/useToast'
import type { Project } from '../../types/foxApi'

const api = useFoxApi()
const toast = useToast()

const props = defineProps<{ project: Project | null }>()

const emit = defineEmits<{
  close: []
  deleted: [id: string]
}>()

async function confirmDelete(): Promise<void> {
  if (!props.project) return
  const target = props.project
  try {
    await api.deleteProject(target.id)
    emit('close')
    toast.success('项目已删除', { message: target.name })
    emit('deleted', target.id)
  } catch (e) {
    toast.error('删除失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  }
}
</script>

<template>
  <Modal :open="project !== null" title="删除项目" width="380px" @close="emit('close')">
    <p class="confirm-hint">
      确认删除「{{ project?.name }}」？项目下的全部接口、环境与示例将一并删除，此操作不可恢复。
    </p>
    <template #footer>
      <button class="rf-btn" type="button" @click="emit('close')">取消</button>
      <button class="rf-btn rf-btn-danger-solid" type="button" :disabled="api.pending.value" @click="confirmDelete">
        删除
      </button>
    </template>
  </Modal>
</template>

<style scoped>
.confirm-hint {
  margin: 0;
  font-size: 12.5px;
  color: var(--text-2);
  line-height: 1.6;
  word-break: break-all;
}
</style>

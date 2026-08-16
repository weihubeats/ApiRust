<script setup lang="ts">
/**
 * ProjectRenameModal：重命名项目弹窗。
 *
 * - project 为 null 时关闭；名称必填（不超过 50 字符）；
 * - 保存成功后 emit('saved', project)，由父级更新列表。
 */
import { ref, watch } from 'vue'
import Modal from '../ui/Modal.vue'
import { useFoxApi } from '../../composables/useFoxApi'
import { useToast } from '../../composables/useToast'
import type { Project } from '../../types/foxApi'

const NAME_MAX = 50

const api = useFoxApi()
const toast = useToast()

const props = defineProps<{ project: Project | null }>()

const emit = defineEmits<{
  close: []
  saved: [project: Project]
}>()

const renameName = ref('')
const renameError = ref<string | null>(null)

// 切换目标项目时重置表单
watch(
  () => props.project,
  (p) => {
    if (p) {
      renameName.value = p.name
      renameError.value = null
    }
  },
)

async function confirmRename(): Promise<void> {
  const name = renameName.value.trim()
  if (!name) {
    renameError.value = '项目名称不能为空'
    return
  }
  if (name.length > NAME_MAX) {
    renameError.value = `项目名称不能超过 ${NAME_MAX} 个字符`
    return
  }
  if (!props.project) return
  try {
    const saved = await api.saveProject({ ...props.project, name, updated_at: new Date().toISOString() })
    emit('close')
    toast.success('项目已重命名')
    emit('saved', saved)
  } catch (e) {
    toast.error('重命名失败', { message: e instanceof Error ? e.message : String(e), duration: 6000 })
  }
}
</script>

<template>
  <Modal :open="project !== null" title="重命名项目" width="420px" @close="emit('close')">
    <div class="form-field">
      <label class="form-label" for="rename-project">项目名称</label>
      <input
        id="rename-project"
        v-model="renameName"
        class="rf-input"
        :class="{ 'rf-input-error': renameError }"
        placeholder="项目名称"
        maxlength="60"
        spellcheck="false"
        @input="renameError = null"
        @keyup.enter="confirmRename"
      />
      <p v-if="renameError" class="rf-field-error" role="alert">{{ renameError }}</p>
    </div>
    <template #footer>
      <button class="rf-btn" type="button" @click="emit('close')">取消</button>
      <button class="rf-btn rf-btn-primary" type="button" :disabled="api.pending.value" @click="confirmRename">
        保存
      </button>
    </template>
  </Modal>
</template>

<style scoped>
.form-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.form-label {
  font-size: 12px;
  color: var(--text-2);
}

.rf-input-error {
  border-color: var(--danger) !important;
}
.rf-input-error:focus {
  border-color: var(--danger) !important;
  box-shadow: 0 0 0 2px var(--danger-tint) !important;
}
</style>

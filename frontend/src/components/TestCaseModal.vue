<script setup lang="ts">
/**
 * TestCaseModal：新建 / 编辑测试用例弹窗（暗黑风）。
 * - 用例名称（必填）
 * - 用例分组：正向 | 负向 | 边界值 | 安全性 | 其他（默认 正向）
 * 确认后 emit('submit', { name, category })，由调用方决定新建 / 编辑语义。
 */
import { ref, watch } from 'vue'
import { TEST_CASE_CATEGORIES } from '../utils/testCases'
import type { TestCaseCategory } from '../types/foxApi'
import CustomSelect from './ui/CustomSelect.vue'
import Modal from './ui/Modal.vue'

const props = defineProps<{
  open: boolean
  title?: string
  /** 预填名称（编辑时传原名称）。 */
  name?: string
  category?: TestCaseCategory
}>()

const emit = defineEmits<{
  'update:open': [open: boolean]
  submit: [payload: { name: string; category: TestCaseCategory }]
}>()

const nameInput = ref('')
const categorySel = ref<TestCaseCategory>('正向')

const CATEGORY_OPTIONS = TEST_CASE_CATEGORIES.map((c) => ({ value: c, label: c }))

function onCategoryChange(value: string | number): void {
  categorySel.value = String(value) as TestCaseCategory
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      nameInput.value = props.name ?? ''
      categorySel.value = props.category ?? '正向'
    }
  },
)

function confirm(): void {
  const trimmed = nameInput.value.trim()
  if (!trimmed) return
  emit('submit', { name: trimmed, category: categorySel.value })
  emit('update:open', false)
}
</script>

<template>
  <Modal
    :open="open"
    :title="title ?? '保存为测试用例'"
    width="360px"
    @update:open="emit('update:open', $event)"
  >
    <div class="tcm">
      <label class="tcm-field">
        <span class="tcm-label">用例名称</span>
        <input
          v-model="nameInput"
          class="tcm-input"
          v-focus-end
          type="text"
          placeholder="如：内部划转-SGB"
          spellcheck="false"
          @keyup.enter="confirm"
        />
      </label>
      <label class="tcm-field">
        <span class="tcm-label">用例分组</span>
        <CustomSelect
          :model-value="categorySel"
          :options="CATEGORY_OPTIONS"
          @update:model-value="onCategoryChange"
        />
      </label>
      <div class="tcm-actions">
        <button class="rf-btn rf-btn-sm" type="button" @click="emit('update:open', false)">
          取消
        </button>
        <button
          class="rf-btn rf-btn-sm rf-btn-primary"
          type="button"
          :disabled="!nameInput.trim()"
          @click="confirm"
        >
          确认
        </button>
      </div>
    </div>
  </Modal>
</template>

<style scoped>
.tcm {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding: 2px 0 4px;
}

.tcm-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tcm-label {
  font-size: 12px;
  color: var(--text-2);
}

.tcm-input {
  padding: 7px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  font-family: inherit;
  font-size: 13px;
  color: var(--text-1);
  background: var(--bg-1);
  outline: none;
  transition: border-color var(--dur) var(--ease);
}
.tcm-input:focus {
  border-color: var(--accent);
}
.tcm-input::placeholder {
  color: var(--text-3);
}

.tcm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
</style>
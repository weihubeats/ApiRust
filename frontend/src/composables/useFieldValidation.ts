/**
 * useFieldValidation：表单字段实时验证（Vue 3 Composable）。
 *
 * 规则在「首次失焦」后才实时校验（避免输入过程中一直红字），
 * 失焦前仅空值检查（必填规则除外——必填在输入时就提示）。
 *
 * 用法：
 * ```ts
 * const name = ref('')
 * const nameField = useFieldValidation(name, [
 *   { rule: 'required', message: '项目名称不能为空' },
 *   { rule: 'maxLength', max: 50, message: '名称不能超过 50 个字符' },
 * ])
 * // 模板：
 * // <input v-model="name" @input="nameField.onInput" @blur="nameField.onBlur" />
 * // <p v-if="nameField.error">{{ nameField.error }}</p>
 * ```
 *
 * `snapshot` 用法（提交时校验整个表单）：
 * ```ts
 * if (nameField.validate()) {
 *   // 通过
 * }
 * ```
 */
import { computed, ref } from 'vue'
import type { Ref } from 'vue'

export type ValidationRule =
  | { rule: 'required'; message: string }
  | { rule: 'minLength'; min: number; message: string }
  | { rule: 'maxLength'; max: number; message: string }
  | { rule: 'pattern'; pattern: RegExp; message: string }
  | { rule: 'url'; message: string }
  | { rule: 'jsonObject'; message: string }
  | { rule: 'custom'; validate: (value: string) => string | null; message: string }

const EMAIL_URL_RE = /^https?:\/\/[^\s]+$/i

function checkRule(value: string, rule: ValidationRule): string | null {
  const v = value.trim()
  switch (rule.rule) {
    case 'required':
      return v ? null : rule.message
    case 'minLength':
      return v.length >= rule.min ? null : rule.message
    case 'maxLength':
      return v.length <= rule.max ? null : rule.message
    case 'pattern':
      return rule.pattern.test(v) ? null : rule.message
    case 'url':
      return EMAIL_URL_RE.test(v) ? null : rule.message
    case 'jsonObject':
      if (!v || v === '{}') return null
      try {
        const parsed = JSON.parse(v)
        return parsed !== null && typeof parsed === 'object' && !Array.isArray(parsed)
          ? null
          : rule.message
      } catch {
        return rule.message
      }
    case 'custom':
      return rule.validate(value) ?? null
  }
}

export function useFieldValidation(value: Ref<string>, rules: ValidationRule[]) {
  const touched = ref(false)
  const dirty = ref(false)
  const firstError = computed(() => {
    if (!touched.value && !dirty.value) return null
    for (const rule of rules) {
      // 必填规则在未输入时也实时提示（配合提交按钮禁用态），其余等失焦。
      if (rule.rule === 'required' || dirty.value) {
        const msg = checkRule(value.value, rule)
        if (msg) return msg
      }
    }
    return null
  })
  const valid = computed(() => firstError.value === null)

  function onInput(): void {
    dirty.value = true
  }

  function onBlur(): void {
    touched.value = true
  }

  /** 提交时手动校验；通过返回 true。 */
  function validate(): boolean {
    touched.value = true
    return valid.value
  }

  /** 重置为未触碰状态。 */
  function reset(): void {
    dirty.value = false
    touched.value = false
  }

  return {
    error: firstError,
    valid,
    touched,
    onInput,
    onBlur,
    validate,
    reset,
  }
}
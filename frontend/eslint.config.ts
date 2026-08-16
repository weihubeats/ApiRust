import vueParser from 'vue-eslint-parser'
import pluginVue from 'eslint-plugin-vue'
import tseslint from 'typescript-eslint'

export default tseslint.config(
  { ignores: ['dist/**', 'node_modules/**', 'src-tauri/**'] },
  ...tseslint.configs.recommended,
  // essential：仅错误级规则；recommended 的模板格式规则与现有代码风格冲突，噪音过大
  ...pluginVue.configs['flat/essential'],
  {
    // .vue 的 <script lang="ts"> 需要显式指定 vue 解析器 + TS 子解析器
    files: ['**/*.vue'],
    languageOptions: {
      parser: vueParser,
      parserOptions: {
        parser: tseslint.parser,
        sourceType: 'module',
      },
    },
  },
  {
    files: ['**/*.{ts,vue}'],
    rules: {
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-vars': [
        'error',
        { argsIgnorePattern: '^_', varsIgnorePattern: '^_' },
      ],
    },
  },
)

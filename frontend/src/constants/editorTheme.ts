/**
 * editorTheme.ts：统一 JSON 编辑器 / 查看器主题（One Dark 色阶）。
 *
 * 颜色唯一事实源：style.css 的 --tok-* CSS 变量（请求 Body 编辑器、
 * 响应 Body 树视图、响应行视图共用同一组色值）。
 *
 * One Dark 色阶：
 * - Key            #e06c75（红）
 * - String         #98c379（绿）
 * - Number         #d19a66（橙）
 * - Boolean/Null   #56b6c2（青，null 斜体）
 * - Punctuation    #abb2bf（灰）
 * - 行号 Gutter    #5c6370（暗灰）
 */

/** JSON 缩进单位（空格数），请求编辑器与响应 Pretty 视图统一。 */
export const EDITOR_INDENT = 2

/** 行号栏结构规范（左右两侧组件共用）。 */
export const LINE_NUMBER_SPEC = {
  /** 行号文字色（--tok-gutter）。 */
  colorVar: 'var(--tok-gutter)',
  /** 行号与正文的左侧间距（px）。 */
  rightPad: 10,
  /** 行号字号（px）。 */
  fontSize: 11,
} as const
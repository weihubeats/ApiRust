<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Compartment, EditorState } from '@codemirror/state'
import { EditorView, highlightActiveLine, highlightActiveLineGutter, highlightSpecialChars, keymap, lineNumbers, placeholder } from '@codemirror/view'
import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete'
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands'
import { json, jsonParseLinter } from '@codemirror/lang-json'
import { linter } from '@codemirror/lint'
import { bracketMatching, defaultHighlightStyle, foldGutter, indentOnInput, syntaxHighlighting, HighlightStyle } from '@codemirror/language'
import { tags } from '@lezer/highlight'

const props = withDefaults(
  defineProps<{
    modelValue: string
    readonly?: boolean
    placeholderText?: string
    autofocus?: boolean
  }>(),
  { readonly: false, placeholderText: '', autofocus: false },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const host = ref<HTMLElement | null>(null)
let view: EditorView | null = null
const readOnlyCompartment = new Compartment()

const darkHighlight = HighlightStyle.define([
  { tag: [tags.propertyName], color: '#c084fc' },
  { tag: [tags.string], color: '#34d399' },
  { tag: [tags.number], color: '#38bdf8' },
  { tag: [tags.bool, tags.null], color: '#fbbf24' },
  { tag: [tags.punctuation, tags.bracket, tags.brace], color: '#94a3b8' },
  { tag: [tags.invalid], color: '#f87171' },
  { tag: [tags.lineComment], color: '#64748b' },
])

const editorTheme = EditorView.theme({
  '&': {
    height: '100%',
    fontSize: '12px',
    color: '#e2e8f0',
  },
  '.cm-scroller': {
    fontFamily: 'var(--font-mono)',
    lineHeight: '1.6',
  },
  '.cm-content': {
    padding: '8px 0',
    caretColor: '#a78bfa',
  },
  '.cm-line': {
    padding: '0 8px',
  },
  '.cm-gutters': {
    backgroundColor: 'transparent',
    borderRight: '1px solid rgba(148, 163, 184, 0.15)',
    color: '#64748b',
  },
  '.cm-foldGutter .cm-gutterElement': {
    cursor: 'pointer',
  },
  '&.cm-focused': {
    outline: 'none',
  },
  '.cm-cursor': {
    borderLeftColor: '#a78bfa',
  },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, ::selection':
    {
      backgroundColor: 'rgba(168, 85, 247, 0.25) !important',
    },
  '.cm-activeLine': {
    backgroundColor: 'rgba(148, 163, 184, 0.06)',
  },
  '.cm-activeLineGutter': {
    backgroundColor: 'rgba(148, 163, 184, 0.06)',
  },
  '.cm-tooltip': {
    backgroundColor: '#1e293b',
    border: '1px solid #334155',
    color: '#e2e8f0',
  },
  '.cm-tooltip-lint': {
    fontSize: '12px',
  },
  '.cm-foldPlaceholder': {
    backgroundColor: 'rgba(148, 163, 184, 0.15)',
    color: '#94a3b8',
    border: 'none',
  },
  '.cm-searchMatch': {
    backgroundColor: 'rgba(251, 191, 36, 0.25)',
  },
})

onMounted(() => {
  if (!host.value) return
  view = new EditorView({
    parent: host.value,
    doc: props.modelValue,
    extensions: [
      lineNumbers(),
      highlightActiveLineGutter(),
      highlightSpecialChars(),
      history(),
      foldGutter(),
      indentOnInput(),
      syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
      bracketMatching(),
      closeBrackets(),
      highlightActiveLine(),
      keymap.of([...closeBracketsKeymap, ...defaultKeymap, ...historyKeymap, indentWithTab]),
      json(),
      syntaxHighlighting(darkHighlight),
      linter(jsonParseLinter()),
      readOnlyCompartment.of(EditorState.readOnly.of(props.readonly)),
      placeholder(props.placeholderText),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          emit('update:modelValue', update.state.doc.toString())
        }
      }),
      editorTheme,
    ],
  })
  if (props.autofocus) view.focus()
})

watch(
  () => props.modelValue,
  (val) => {
    if (!view) return
    const current = view.state.doc.toString()
    if (current !== val) {
      view.dispatch({ changes: { from: 0, to: current.length, insert: val } })
    }
  },
)

watch(
  () => props.readonly,
  (val) => {
    view?.dispatch({ effects: readOnlyCompartment.reconfigure(EditorState.readOnly.of(val)) })
  },
)

function requestMeasure(): void {
  view?.requestMeasure()
}

defineExpose({ requestMeasure, focus: () => view?.focus() })

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})
</script>

<template>
  <div ref="host" class="cm-host"></div>
</template>

<style scoped>
.cm-host {
  height: 100%;
  min-height: 0;
  overflow: hidden;
}
.cm-host :deep(.cm-editor) {
  height: 100%;
}
.cm-host :deep(.cm-scroller) {
  overflow: auto;
}
</style>
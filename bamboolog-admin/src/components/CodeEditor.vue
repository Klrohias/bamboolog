<template>
  <div class="code-editor" :style="{ height }">
    <div ref="host" class="code-editor-host"></div>
  </div>
</template>

<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as monaco from 'monaco-editor/editor/editor.api'
import 'monaco-editor/editor/editor.main'
import editorWorker from 'monaco-editor/editor/editor.worker.js?worker'
import jsonWorker from 'monaco-editor/language/json/json.worker.js?worker'
import 'monaco-editor/language/json/monaco.contribution'
import 'monaco-editor/basic-languages/monaco.contribution'
import { useSettingsStore } from '@/stores/settings'

const props = withDefaults(defineProps<{
  modelValue: string
  language?: string
  height?: string
}>(), {
  language: 'plaintext',
  height: '560px',
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

const settingsStore = useSettingsStore()

const monacoGlobal = self as typeof globalThis & {
  MonacoEnvironment?: { getWorker: (_workerId: string, label: string) => Worker }
}
monacoGlobal.MonacoEnvironment = {
  getWorker(_workerId, label) {
    return label === 'json' ? new jsonWorker() : new editorWorker()
  }
}

const host = ref<HTMLElement | null>(null)
let editor: monaco.editor.IStandaloneCodeEditor | undefined
let model: monaco.editor.ITextModel | undefined

function syncEditorTheme(theme: 'light' | 'dark') {
  monaco.editor.setTheme(theme === 'dark' ? 'vs-dark' : 'vs')
}

async function createEditor() {
  await nextTick()
  if (!host.value || editor) return

  syncEditorTheme(settingsStore.theme)
  model = monaco.editor.createModel(props.modelValue, props.language)
  editor = monaco.editor.create(host.value, {
    model,
    automaticLayout: true,
    formatOnPaste: true,
    formatOnType: true,
    minimap: { enabled: false },
    tabSize: 2,
    contextmenu: true,
  })
  editor.onDidChangeModelContent(() => {
    emit('update:modelValue', editor?.getValue() ?? '')
  })
  editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyMod.Shift | monaco.KeyCode.KeyP, () => {
    editor?.getAction('editor.action.quickCommand')?.run()
  })
}

watch(() => props.modelValue, value => {
  if (editor && editor.getValue() !== value) editor.setValue(value)
})

watch(() => props.language, language => {
  if (model) monaco.editor.setModelLanguage(model, language)
})

watch(() => settingsStore.theme, syncEditorTheme)

onMounted(createEditor)
onBeforeUnmount(() => {
  editor?.dispose()
  model?.dispose()
})
</script>

<style scoped>
.code-editor {
  border: 1px solid var(--n-border-color);
}

.code-editor-host {
  height: 100%;
}
</style>

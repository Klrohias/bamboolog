<template>
  <div ref="editorRef" class="markdown-editor"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from 'vue'
import Editor from '@toast-ui/editor'
import '@toast-ui/editor/dist/toastui-editor.css'

const props = defineProps<{
  modelValue: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const editorRef = ref<HTMLElement | null>(null)
let editor: Editor | null = null

onMounted(() => {
  if (editorRef.value) {
    editor = new Editor({
      el: editorRef.value,
      height: '500px',
      initialEditType: 'markdown',
      previewStyle: 'vertical',
      initialValue: props.modelValue,
      theme: 'dark', // We can make this dynamic if needed
      events: {
        change: () => {
          if (editor) {
            emit('update:modelValue', editor.getMarkdown())
          }
        },
      },
    })
  }
})

watch(
  () => props.modelValue,
  (newValue) => {
    if (editor && newValue !== editor.getMarkdown()) {
      editor.setMarkdown(newValue)
    }
  }
)

onBeforeUnmount(() => {
  if (editor) {
    editor.destroy()
  }
})
</script>

<style>
/* Toast UI Editor dark theme adjustments if needed */
.markdown-editor {
  background-color: white; /* Default white, but toast ui has its own theme */
  border-radius: 8px;
  overflow: hidden;
}

.toastui-editor-defaultUI {
  border: 1px solid #d1d5db !important;
}
</style>

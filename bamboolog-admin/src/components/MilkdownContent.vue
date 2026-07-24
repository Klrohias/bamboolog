<template>
  <div class="milkdown-editor">
    <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { Crepe } from '@milkdown/crepe'
import '@milkdown/crepe/theme/common/style.css'
import '@milkdown/crepe/theme/frame.css'
import { Milkdown, useEditor } from '@milkdown/vue'
import { attachmentApi } from '@/api/attachments'

const props = defineProps<{
  modelValue: string
  storageEngineId?: number
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'uploadError'): void
}>()

async function uploadImage(file: File): Promise<string> {
  try {
    const { data } = await attachmentApi.upload(file, props.storageEngineId)
    return `/attachments/${data.data.hash}`
  } catch (error) {
    emit('uploadError')
    throw error
  }
}

useEditor((root) => {
  const editor = new Crepe({
    root,
    defaultValue: props.modelValue,
    features: {
      [Crepe.Feature.BlockEdit]: false,
      [Crepe.Feature.TopBar]: true
    },
    featureConfigs: {
      [Crepe.Feature.ImageBlock]: {
        onUpload: uploadImage
      }
    }
  })

  editor.on((listener) => {
    listener.markdownUpdated((_ctx, markdown) => {
      if (markdown !== props.modelValue) {
        emit('update:modelValue', markdown)
      }
    })
  })

  return editor
})
</script>

<style scoped>
.milkdown-editor {
  width: 100%;
  min-width: 0;
  max-width: 100%;
  min-height: 480px;
  overflow-x: clip;
  border: 1px solid var(--n-border-color);
  background: var(--n-color);
}

.milkdown-editor :deep([data-milkdown-root]),
.milkdown-editor :deep(.milkdown) {
  width: 100%;
  min-width: 0;
  min-height: 480px;
}

.milkdown-editor :deep(.ProseMirror) {
  min-height: 420px;
  padding: 28px;
  outline: none;
}

.milkdown-editor :deep(.milkdown-top-bar) {
  max-width: 100%;
}

@media (max-width: 640px) {
  .milkdown-editor :deep(.ProseMirror) {
    padding: 20px;
  }
}
</style>

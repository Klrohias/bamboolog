<template>
  <div class="milkdown-editor" :class="{ 'milkdown-editor--dark': settingsStore.theme === 'dark' }">
    <Milkdown />
  </div>
</template>

<script setup lang="ts">
import { Crepe } from '@milkdown/crepe'
import '@milkdown/crepe/theme/common/style.css'
import '@milkdown/crepe/theme/frame.css'
import { Milkdown, useEditor } from '@milkdown/vue'
import { attachmentApi } from '@/api/attachments'
import { useSettingsStore } from '@/stores/settings'

const props = defineProps<{
  modelValue: string
  storageEngineId?: number
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'uploadError'): void
}>()

const settingsStore = useSettingsStore()

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

.milkdown-editor--dark :deep(.milkdown) {
  --crepe-color-background: #1a1a1a;
  --crepe-color-on-background: #e6e6e6;
  --crepe-color-surface: #121212;
  --crepe-color-surface-low: #1c1c1c;
  --crepe-color-on-surface: #d1d1d1;
  --crepe-color-on-surface-variant: #a9a9a9;
  --crepe-color-outline: #757575;
  --crepe-color-primary: #b5b5b5;
  --crepe-color-secondary: #4d4d4d;
  --crepe-color-on-secondary: #d6d6d6;
  --crepe-color-inverse: #e5e5e5;
  --crepe-color-on-inverse: #2a2a2a;
  --crepe-color-inline-code: #ff6666;
  --crepe-color-error: #ff6666;
  --crepe-color-hover: #232323;
  --crepe-color-selected: #2f2f2f;
  --crepe-color-inline-area: #2b2b2b;
}

@media (max-width: 640px) {
  .milkdown-editor :deep(.ProseMirror) {
    padding: 20px;
  }
}
</style>

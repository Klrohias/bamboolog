<template>
  <MilkdownProvider>
    <MilkdownContent
      :model-value="modelValue"
      :storage-engine-id="storageEngineId"
      @update:model-value="emit('update:modelValue', $event)"
      @upload-error="emit('uploadError')"
    />
  </MilkdownProvider>
</template>

<script setup lang="ts">
import { defineAsyncComponent } from 'vue'
import { MilkdownProvider } from '@milkdown/vue'

const MilkdownContent = defineAsyncComponent(() => import('./MilkdownContent.vue'))

defineProps<{
  modelValue: string
  storageEngineId?: number
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
  (e: 'uploadError'): void
}>()

</script>

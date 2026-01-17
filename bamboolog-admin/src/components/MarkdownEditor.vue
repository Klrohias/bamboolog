<template>
  <n-input
    v-model:value="content"
    type="textarea"
    :placeholder="$t('posts.content')"
    :autosize="{ minRows: 20 }"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { NInput } from 'naive-ui'

const props = defineProps<{
  modelValue: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: string): void
}>()

const content = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value)
})
</script>

<style scoped>
</style>

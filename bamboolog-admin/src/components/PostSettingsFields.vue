<template>
  <n-form-item :label="$t('posts.slug')" path="name">
    <n-input v-model:value="model.name" :placeholder="$t('posts.slug')" />
  </n-form-item>
  <n-form-item :label="$t('posts.created_at')">
    <n-date-picker v-model:value="model.created_at" type="datetime" style="width: 100%" />
  </n-form-item>
  <n-form-item :label="$t('posts.updated_at')">
    <n-date-picker v-model:value="model.updated_at" type="datetime" style="width: 100%" />
  </n-form-item>
  <n-form-item :label="$t('posts.description')">
    <n-input v-model:value="model.description" type="textarea" :autosize="{ minRows: 2, maxRows: 4 }"
      :placeholder="$t('posts.description_placeholder')" />
  </n-form-item>
  <n-form-item :label="$t('posts.illustration')">
    <n-input v-model:value="model.illustration" :placeholder="$t('posts.illustration_placeholder')" />
  </n-form-item>
  <n-form-item :label="$t('posts.categories')">
    <n-dynamic-tags v-model:value="model.categories" />
  </n-form-item>
  <n-form-item :label="$t('posts.tags')">
    <n-dynamic-tags v-model:value="model.tags" />
  </n-form-item>
  <n-form-item :label="$t('posts.hidden')">
    <n-switch v-model:value="model.hidden" />
  </n-form-item>
  <n-form-item :label="$t('posts.image_storage')">
    <n-select v-model:value="storageEngineId" clearable :options="storageEngineOptions"
      :placeholder="$t('posts.default_storage')" />
  </n-form-item>
</template>

<script setup lang="ts">
export interface PostSettingsForm {
  name: string
  created_at: number | null
  updated_at: number | null
  description: string
  illustration: string
  categories: string[]
  tags: string[]
  hidden: boolean
}

defineProps<{
  storageEngineOptions: Array<{ label: string, value: number }>
}>()

const model = defineModel<PostSettingsForm>('model', { required: true })
const storageEngineId = defineModel<number | null>('storageEngineId', { required: true })
</script>

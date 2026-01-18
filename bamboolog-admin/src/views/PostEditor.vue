<template>
  <n-space vertical size="large">
    <n-page-header :title="isEdit ? $t('posts.edit_post') : $t('posts.new_post')" @back="$router.push('/posts')">
    </n-page-header>

    <n-card>
      <n-form :model="form" ref="formRef" :rules="rules">
        <n-form-item :label="$t('posts.title')" path="title">
          <n-input v-model:value="form.title" :placeholder="$t('posts.title')" />
        </n-form-item>
        <n-form-item :label="$t('posts.slug')" path="name">
          <n-input v-model:value="form.name" :placeholder="$t('posts.slug')" />
        </n-form-item>
        <n-form-item :label="$t('posts.content')" path="content">
          <MarkdownEditor v-model="form.content" style="width: 100%" />
        </n-form-item>
        <n-form-item>
          <n-space>
            <n-button type="primary" :loading="saving" @click="handleSave">{{ $t('common.save') }}</n-button>
            <n-button @click="$router.push('/posts')">{{ $t('common.cancel') }}</n-button>
          </n-space>
        </n-form-item>
      </n-form>
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage, type FormInst } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { postsApi } from '@/api/posts'
import MarkdownEditor from '@/components/MarkdownEditor.vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const message = useMessage()
const formRef = ref<FormInst | null>(null)

const isEdit = computed(() => !!route.params.id)
const saving = ref(false)

const form = ref({
  title: '',
  name: '',
  content: ''
})

const rules = {
  title: { required: true, message: () => t('posts.title'), trigger: 'blur' },
  name: { required: true, message: () => t('posts.slug'), trigger: 'blur' },
  content: { required: true, message: () => t('posts.content'), trigger: 'blur' }
}

async function fetchPost() {
  if (!isEdit.value) return
  try {
    const { data } = await postsApi.get(Number(route.params.id))
    const post = data.data
    form.value.title = post.title
    form.value.name = post.name
    form.value.content = post.content
  } catch (e: any) {
    message.error(t('posts.fetch_failed'))
    router.push('/posts')
  }
}

async function handleSave() {
  await formRef.value?.validate()
  saving.value = true
  try {
    if (isEdit.value) {
      await postsApi.update(Number(route.params.id), form.value)
      message.success(t('posts.update_success'))
    } else {
      await postsApi.create(form.value)
      message.success(t('posts.create_success'))
    }
    router.push('/posts')
  } catch (e: any) {
    message.error(e.response?.data?.message || t('common.error'))
  } finally {
    saving.value = false
  }
}

onMounted(fetchPost)
</script>


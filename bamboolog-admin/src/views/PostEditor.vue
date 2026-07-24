<template>
  <n-space vertical size="large">
    <n-page-header :title="isEdit ? $t('posts.edit_post') : $t('posts.new_post')" @back="$router.push('/posts')">
    </n-page-header>

    <n-card>
      <n-form :model="form" ref="formRef" :rules="rules">
        <n-form-item :label="$t('posts.title')" path="title">
          <n-input v-model:value="form.title" :placeholder="$t('posts.title')" />
        </n-form-item>
        <n-form-item :label="$t('posts.content')" path="content">
          <n-spin v-if="!editorReady" size="small" />
          <MarkdownEditor
            v-else
            v-model="form.content"
            :storage-engine-id="storageEngineId ?? undefined"
            style="width: 100%"
            @upload-error="message.error(t('posts.image_upload_failed'))"
          />
        </n-form-item>
        <n-form-item :label="$t('posts.slug')" path="name">
          <n-input v-model:value="form.name" :placeholder="$t('posts.slug')" />
        </n-form-item>
        <n-form-item :label="$t('posts.description')">
          <n-input v-model:value="form.description" type="textarea" :autosize="{ minRows: 2, maxRows: 4 }" :placeholder="$t('posts.description_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('posts.illustration')">
          <n-input v-model:value="form.illustration" :placeholder="$t('posts.illustration_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('posts.categories')">
          <n-dynamic-tags v-model:value="form.categories" />
        </n-form-item>
        <n-form-item :label="$t('posts.tags')">
          <n-dynamic-tags v-model:value="form.tags" />
        </n-form-item>
        <n-form-item :label="$t('posts.hidden')">
          <n-switch v-model:value="form.hidden" />
        </n-form-item>
        <n-form-item :label="$t('posts.image_storage')">
          <n-select
            v-model:value="storageEngineId"
            clearable
            :options="storageEngineOptions"
            :placeholder="$t('posts.default_storage')"
          />
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
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage, type FormInst } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { postsApi } from '@/api/posts'
import { storageApi, type StorageEngine } from '@/api/storage'
import MarkdownEditor from '@/components/MarkdownEditor.vue'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const message = useMessage()
const formRef = ref<FormInst | null>(null)

const isEdit = computed(() => !!route.params.id)
const saving = ref(false)
const editorReady = ref(false)
const storageEngines = ref<StorageEngine[]>([])
const storageEngineId = ref<number | null>(null)

const storageEngineOptions = computed(() =>
  storageEngines.value
    .filter(engine => engine.enabled)
    .map(engine => ({ label: engine.name, value: engine.id }))
)

const form = ref({
  title: '',
  name: '',
  content: '',
  description: '',
  illustration: '',
  categories: [] as string[],
  tags: [] as string[],
  hidden: false
})

const rules = {
  title: { required: true, message: () => t('posts.title'), trigger: 'blur' },
  name: { required: true, message: () => t('posts.slug'), trigger: 'blur' },
  content: { required: true, message: () => t('posts.content'), trigger: 'blur' }
}

async function fetchPost() {
  editorReady.value = false
  form.value = { title: '', name: '', content: '', description: '', illustration: '', categories: [], tags: [], hidden: false }
  if (!isEdit.value) {
    editorReady.value = true
    return
  }

  try {
    const { data } = await postsApi.get(Number(route.params.id))
    const post = data.data
    form.value.title = post.title
    form.value.name = post.name
    form.value.content = post.content
    form.value.description = post.description || ''
    form.value.illustration = post.illustration || ''
    form.value.categories = post.categories || []
    form.value.tags = post.tags || []
    form.value.hidden = post.hidden || false
  } catch (e: any) {
    message.error(t('posts.fetch_failed'))
    router.push('/posts')
  } finally {
    editorReady.value = true
  }
}

async function fetchStorageEngines() {
  try {
    const { data } = await storageApi.list()
    storageEngines.value = data.data
    const defaultEngine = storageEngines.value.find(engine => engine.enabled && engine.is_default)
    storageEngineId.value = defaultEngine?.id ?? storageEngineOptions.value[0]?.value ?? null
  } catch {
    message.warning(t('posts.storage_load_failed'))
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

watch(() => route.params.id, fetchPost, { immediate: true })
onMounted(fetchStorageEngines)
</script>

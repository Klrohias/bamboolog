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
    <n-space vertical :size="8" class="illustration-field">
      <n-image
        v-if="model.illustration"
        class="illustration-preview"
        :src="model.illustration"
        object-fit="cover"
        preview-disabled
      />
      <n-button @click="openIllustrationDialog">{{ $t('posts.illustration_change') }}</n-button>
    </n-space>
  </n-form-item>
  <n-form-item :label="$t('posts.categories')">
    <n-dynamic-tags v-model:value="model.categories" />
  </n-form-item>
  <n-form-item :label="$t('posts.tags')">
    <n-dynamic-tags v-model:value="model.tags" />
  </n-form-item>
  <n-form-item :label="$t('posts.functions')">
    <n-dynamic-tags v-model:value="model.functions" />
  </n-form-item>
  <n-form-item :label="$t('posts.hidden')">
    <n-switch v-model:value="model.hidden" />
  </n-form-item>
  <n-form-item :label="$t('posts.image_storage')">
    <n-select v-model:value="storageEngineId" clearable :options="storageEngineOptions"
      :placeholder="$t('posts.default_storage')" />
  </n-form-item>

  <n-modal
    v-model:show="illustrationDialogVisible"
    preset="card"
    :title="$t('posts.illustration_upload_title')"
    style="width: min(560px, calc(100vw - 32px))"
    :mask-closable="!uploadingIllustration"
    :closable="!uploadingIllustration"
    @after-leave="resetIllustrationUpload"
  >
    <n-upload
      v-model:file-list="illustrationFiles"
      :default-upload="false"
      :max="1"
      accept="image/*"
      :disabled="uploadingIllustration"
      :on-before-upload="validateIllustrationUpload"
    >
      <n-upload-dragger>
        <div class="illustration-upload-icon">
          <n-icon size="48" :depth="3"><image-outline /></n-icon>
        </div>
        <div>{{ $t('posts.illustration_upload_hint') }}</div>
      </n-upload-dragger>
    </n-upload>
    <template #footer>
      <n-space justify="end">
        <n-button :disabled="uploadingIllustration" @click="illustrationDialogVisible = false">{{ $t('common.cancel') }}</n-button>
        <n-button type="primary" :loading="uploadingIllustration" :disabled="illustrationFiles.length !== 1" @click="confirmIllustrationUpload">{{ $t('common.confirm') }}</n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useMessage, type UploadFileInfo } from 'naive-ui'
import { ImageOutline } from '@vicons/ionicons5'
import { useI18n } from 'vue-i18n'
import { attachmentApi } from '@/api/attachments'

export interface PostSettingsForm {
  name: string
  created_at: number | null
  updated_at: number | null
  description: string
  illustration: string
  categories: string[]
  tags: string[]
  functions: string[]
  hidden: boolean
}

defineProps<{
  storageEngineOptions: Array<{ label: string, value: number }>
}>()

const model = defineModel<PostSettingsForm>('model', { required: true })
const storageEngineId = defineModel<number | null>('storageEngineId', { required: true })

const { t } = useI18n()
const message = useMessage()
const illustrationDialogVisible = ref(false)
const uploadingIllustration = ref(false)
const illustrationFiles = ref<UploadFileInfo[]>([])

function openIllustrationDialog() {
  illustrationDialogVisible.value = true
}

function resetIllustrationUpload() {
  illustrationFiles.value = []
}

function validateIllustrationFile(file: UploadFileInfo) {
  const selectedFile = file.file
  const isImage = selectedFile?.type.startsWith('image/') ?? false
  if (!isImage) message.error(t('posts.illustration_image_only'))
  return isImage
}

function validateIllustrationUpload({ file }: { file: UploadFileInfo }) {
  return validateIllustrationFile(file)
}

async function confirmIllustrationUpload() {
  const selectedFile = illustrationFiles.value[0]
  const file = selectedFile?.file
  if (!selectedFile || !file || !validateIllustrationFile(selectedFile)) return

  uploadingIllustration.value = true
  try {
    const { data } = await attachmentApi.upload(file, storageEngineId.value ?? undefined)
    model.value.illustration = `/attachments/${data.data.hash}`
    illustrationDialogVisible.value = false
  } catch {
    message.error(t('posts.image_upload_failed'))
  } finally {
    uploadingIllustration.value = false
  }
}
</script>

<style scoped>
.illustration-field {
  width: 100%;
}

.illustration-preview {
  display: block;
  width: 100%;
  height: 160px;
}

.illustration-preview :deep(img) {
  width: 100%;
  height: 160px;
}

.illustration-upload-icon {
  margin-bottom: 12px;
}
</style>

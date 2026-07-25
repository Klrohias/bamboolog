<template>
  <n-space vertical size="large">
    <n-page-header :title="$t('themes.title')" :subtitle="$t('themes.subtitle')">
      <template #extra>
        <n-button type="primary" @click="uploadVisible = true">{{ $t('themes.upload') }}</n-button>
      </template>
    </n-page-header>

    <n-spin :show="loading">
      <n-grid v-if="themes.length" cols="1 s:2 l:3" :x-gap="16" :y-gap="16" responsive="screen">
        <n-gi v-for="theme in themes" :key="theme.id">
          <n-card class="theme-card" :segmented="{ content: true, footer: true }">
            <template #header>
              <n-space align="center" :size="10">
                <span class="theme-name">{{ displayName(theme) }}</span>
                <n-tag v-if="theme.version" size="small" :bordered="false">v{{ theme.version }}</n-tag>
              </n-space>
            </template>
            <template #header-extra>
              <n-tag :type="theme.active ? 'success' : 'default'" size="small">{{ theme.active ? $t('themes.active') : $t('themes.inactive') }}</n-tag>
            </template>

            <p class="theme-description">{{ theme.description || $t('themes.no_description') }}</p>
            <div class="theme-meta">{{ theme.author || $t('themes.unknown_author') }}</div>

            <template #footer>
              <n-space justify="end">
                <n-button @click="openDetails(theme)">{{ $t('themes.details') }}</n-button>
                <n-button v-if="theme.active" type="primary" @click="router.push('/theme-settings')">{{ $t('themes.configure') }}</n-button>
                <template v-else>
                  <n-popconfirm
                    :positive-text="$t('common.confirm')"
                    :negative-text="$t('common.cancel')"
                    @positive-click="deleteTheme(theme)"
                  >
                    <template #trigger>
                      <n-button type="error" :loading="deleting === theme.id">{{ $t('themes.delete') }}</n-button>
                    </template>
                    {{ $t('themes.delete_confirm', { name: displayName(theme) }) }}
                  </n-popconfirm>
                  <n-button type="primary" :loading="activating === theme.id" :disabled="deleting === theme.id" @click="activate(theme)">{{ $t('themes.activate') }}</n-button>
                </template>
              </n-space>
            </template>
          </n-card>
        </n-gi>
      </n-grid>
      <n-empty v-else-if="!loading" :description="$t('themes.empty')" />
    </n-spin>

    <n-modal v-model:show="detailsVisible" preset="card" :title="selectedTheme ? displayName(selectedTheme) : ''" style="width: min(560px, calc(100vw - 32px))">
      <n-descriptions v-if="selectedTheme" :column="1" label-placement="left" bordered>
        <n-descriptions-item :label="$t('themes.identifier')">{{ selectedTheme.id }}</n-descriptions-item>
        <n-descriptions-item :label="$t('themes.version')">{{ selectedTheme.version || '-' }}</n-descriptions-item>
        <n-descriptions-item :label="$t('themes.author')">{{ selectedTheme.author || '-' }}</n-descriptions-item>
        <n-descriptions-item :label="$t('themes.description')">{{ selectedTheme.description || '-' }}</n-descriptions-item>
        <n-descriptions-item v-if="selectedTheme.homepage" :label="$t('themes.homepage')"><a :href="selectedTheme.homepage" target="_blank" rel="noreferrer">{{ selectedTheme.homepage }}</a></n-descriptions-item>
      </n-descriptions>
    </n-modal>

    <n-modal v-model:show="uploadVisible" preset="card" :title="$t('themes.upload_title')" style="width: min(560px, calc(100vw - 32px))" :mask-closable="!uploading" :closable="!uploading">
      <n-upload
        v-model:file-list="uploadFiles"
        :default-upload="false"
        :max="1"
        accept=".zip,application/zip,application/x-zip-compressed"
        :disabled="uploading"
        :on-before-upload="validateUpload"
      >
        <n-upload-dragger>
          <div class="upload-icon">
            <n-icon size="48" :depth="3">
              <ArchiveOutline />
            </n-icon>
          </div>
          <div class="upload-text">{{ $t('themes.upload_hint') }}</div>
          <div class="upload-help">{{ $t('themes.upload_limit') }}</div>
        </n-upload-dragger>
      </n-upload>
      <template #footer>
        <n-space justify="end">
          <n-button :disabled="uploading" @click="uploadVisible = false">{{ $t('common.cancel') }}</n-button>
          <n-button type="primary" :loading="uploading" :disabled="uploadFiles.length !== 1" @click="uploadTheme">{{ $t('common.confirm') }}</n-button>
        </n-space>
      </template>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useMessage, type UploadFileInfo } from 'naive-ui'
import { ArchiveOutline } from '@vicons/ionicons5'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { themesApi, type ThemeDetails } from '@/api/themes'

const { t } = useI18n()
const message = useMessage()
const router = useRouter()
const themes = ref<ThemeDetails[]>([])
const loading = ref(false)
const activating = ref<string | null>(null)
const deleting = ref<string | null>(null)
const detailsVisible = ref(false)
const selectedTheme = ref<ThemeDetails | null>(null)
const uploadVisible = ref(false)
const uploading = ref(false)
const uploadFiles = ref<UploadFileInfo[]>([])

function displayName(theme: ThemeDetails) {
  return theme.name || theme.id
}

function openDetails(theme: ThemeDetails) {
  selectedTheme.value = theme
  detailsVisible.value = true
}

async function fetchThemes() {
  loading.value = true
  try {
    const { data } = await themesApi.list()
    themes.value = data.data || []
  } catch {
    message.error(t('themes.fetch_failed'))
  } finally {
    loading.value = false
  }
}

async function activate(theme: ThemeDetails) {
  activating.value = theme.id
  try {
    await themesApi.activate(theme.id)
    await fetchThemes()
    message.success(t('themes.activate_success'))
  } catch {
    message.error(t('themes.activate_failed'))
  } finally {
    activating.value = null
  }
}

async function deleteTheme(theme: ThemeDetails) {
  deleting.value = theme.id
  try {
    await themesApi.delete(theme.id)
    themes.value = themes.value.filter(item => item.id !== theme.id)
    message.success(t('themes.delete_success'))
  } catch {
    message.error(t('themes.delete_failed'))
  } finally {
    deleting.value = null
  }
}

function validateThemeArchive(file: UploadFileInfo) {
  const name = file.file?.name ?? file.name ?? ''
  const isZip = name.toLowerCase().endsWith('.zip')
  const isWithinLimit = file.file !== null && file.file !== undefined && file.file.size <= 15 * 1024 * 1024
  if (!isZip) message.error(t('themes.upload_zip_only'))
  else if (!isWithinLimit) message.error(t('themes.upload_too_large'))
  return isZip && isWithinLimit
}

function validateUpload({ file }: { file: UploadFileInfo }) {
  return validateThemeArchive(file)
}

async function uploadTheme() {
  const selectedFile = uploadFiles.value[0]
  const file = selectedFile?.file
  if (!selectedFile || !file || !validateThemeArchive(selectedFile)) return
  uploading.value = true
  try {
    await themesApi.upload(file)
    uploadVisible.value = false
    uploadFiles.value = []
    await fetchThemes()
    message.success(t('themes.upload_success'))
  } catch {
    message.error(t('themes.upload_failed'))
  } finally {
    uploading.value = false
  }
}

onMounted(fetchThemes)
</script>

<style scoped>
.theme-card {
  max-width: 920px;
}

.theme-name {
  font-size: 17px;
  font-weight: 600;
}

.theme-description {
  min-height: 24px;
  margin: 0 0 12px;
  line-height: 1.5;
}

.theme-meta {
  color: var(--n-text-color-3);
  font-size: 13px;
}

.upload-text {
  font-size: 15px;
}

.upload-icon {
  margin-bottom: 12px;
}

.upload-help {
  margin-top: 8px;
  color: var(--n-text-color-3);
  font-size: 13px;
}
</style>

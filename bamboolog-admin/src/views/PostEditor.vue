<template>
  <n-form class="post-editor" :model="form" ref="formRef">
    <n-layout class="editor-layout" :has-sider="isDesktop" sider-placement="right">
      <n-layout-content class="editor-workspace">
        <header class="editor-toolbar">
          <n-button class="editor-back" quaternary @click="$router.push('/posts')">
            <template #icon>
              <n-icon><arrow-back-outline /></n-icon>
            </template>
            {{ isEdit ? $t('posts.edit_post') : $t('posts.new_post') }}
          </n-button>
          <n-space size="small" align="center">
            <n-switch v-model:value="useCodeEditor" :disabled="!editorReady">
              <template #checked>{{ $t('posts.editor_monaco') }}</template>
              <template #unchecked>{{ $t('posts.editor_milkdown') }}</template>
            </n-switch>
            <n-button type="primary" :loading="saving" @click="handleSave">{{ $t('common.save') }}</n-button>
            <n-tooltip v-if="isDesktop && settingsCollapsed">
              <template #trigger>
                <n-button quaternary circle :aria-label="settingsToggleLabel" @click="settingsCollapsed = false">
                  <template #icon>
                    <n-icon><chevron-back-outline /></n-icon>
                  </template>
                </n-button>
              </template>
              {{ settingsToggleLabel }}
            </n-tooltip>
          </n-space>
        </header>

        <n-input v-model:value="form.title" class="editor-title-input" :bordered="false" size="large"
          :placeholder="$t('posts.title')" />
        <div class="editor-content-field">
          <n-spin v-if="!editorReady" class="editor-loading" size="small" />
          <MarkdownEditor v-else-if="!useCodeEditor" v-model="form.content" :storage-engine-id="storageEngineId ?? undefined"
            @upload-error="message.error(t('posts.image_upload_failed'))" />
          <CodeEditor v-else v-model="form.content" language="markdown" />
        </div>
      </n-layout-content>

      <n-layout-sider v-if="isDesktop" class="post-settings-sidebar" bordered collapse-mode="transform"
        :collapsed-width="0" :width="360" :collapsed="settingsCollapsed">
        <n-affix class="post-settings-affix" :top="64" :trigger-top="64">
          <header class="post-settings-header">
            <span>{{ $t('common.post_settings') }}</span>
            <n-tooltip>
              <template #trigger>
                <n-button quaternary circle :aria-label="settingsToggleLabel" @click="settingsCollapsed = true">
                  <template #icon>
                    <n-icon><chevron-forward-outline /></n-icon>
                  </template>
                </n-button>
              </template>
              {{ settingsToggleLabel }}
            </n-tooltip>
          </header>
          <div class="post-settings-fields">
            <PostSettingsFields v-model:model="form" v-model:storage-engine-id="storageEngineId"
              :storage-engine-options="storageEngineOptions" />
          </div>
        </n-affix>
      </n-layout-sider>
    </n-layout>

    <section v-if="!isDesktop" class="post-settings post-settings-mobile">
      <n-card :title="$t('common.post_settings')">
        <PostSettingsFields v-model:model="form" v-model:storage-engine-id="storageEngineId"
          :storage-engine-options="storageEngineOptions" />
      </n-card>
    </section>
  </n-form>
</template>

<script setup lang="ts">
import { ref, computed, defineAsyncComponent, onMounted, onBeforeUnmount, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useMessage, type FormInst } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { ArrowBackOutline, ChevronBackOutline, ChevronForwardOutline } from '@vicons/ionicons5'
import { postsApi } from '@/api/posts'
import { storageApi, type StorageEngine } from '@/api/storage'
import MarkdownEditor from '@/components/MarkdownEditor.vue'
import PostSettingsFields from '@/components/PostSettingsFields.vue'

const CodeEditor = defineAsyncComponent(() => import('@/components/CodeEditor.vue'))

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const message = useMessage()
const formRef = ref<FormInst | null>(null)

const isEdit = computed(() => !!route.params.id)
const saving = ref(false)
const editorReady = ref(false)
const useCodeEditor = ref(false)
const storageEngines = ref<StorageEngine[]>([])
const storageEngineId = ref<number | null>(null)
const isDesktop = ref(false)
const settingsCollapsed = ref(true)
let desktopQuery: MediaQueryList | undefined

function handleDesktopQueryChange(event: MediaQueryListEvent) {
  isDesktop.value = event.matches
}

const settingsToggleLabel = computed(() =>
  settingsCollapsed.value ? t('common.expand') : t('common.collapse')
)

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
  if (!form.value.title.trim()) {
    message.error(t('posts.title'))
    return
  }
  if (!form.value.content.trim()) {
    message.error(t('posts.content'))
    return
  }
  if (!form.value.name.trim()) {
    form.value.name = `post-${crypto.randomUUID()}`
  }

  try {
    await formRef.value?.validate()
  } catch {
    return
  }

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
onMounted(() => {
  fetchStorageEngines()
  desktopQuery = window.matchMedia('(min-width: 1024px)')
  isDesktop.value = desktopQuery.matches
  desktopQuery.addEventListener('change', handleDesktopQueryChange)
})

onBeforeUnmount(() => {
  desktopQuery?.removeEventListener('change', handleDesktopQueryChange)
})
</script>

<style scoped>
.editor-layout {
  min-height: calc(100vh - 64px);
}

.editor-workspace {
  min-height: calc(100vh - 64px);
  background: var(--n-color);
}

.editor-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 12px;
}

.editor-back {
  font-size: 16px;
  font-weight: 600;
}

.editor-title-input {
  width: 100%;
}

.editor-title-input :deep(.n-input-wrapper) {
  padding: 24px 40px;
  border-radius: 0;
  background: transparent;
}

.editor-title-input :deep(.n-input-wrapper::before),
.editor-title-input :deep(.n-input-wrapper::after) {
  background-color: transparent;
}

.editor-title-input :deep(.n-input__input-el) {
  font-size: 28px;
  font-weight: 600;
}

.editor-content-field :deep(.n-spin-container) {
  width: 100%;
}

.editor-loading {
  display: flex;
  min-height: 480px;
  align-items: center;
  justify-content: center;
}

.post-settings-sidebar {
  overflow-y: auto;
  background: var(--n-color);
}

.post-settings-affix {
  width: 360px;
  max-height: calc(100vh - 64px);
  overflow-y: auto;
  background: var(--n-color);
}

.post-settings-header {
  display: flex;
  min-height: 56px;
  align-items: center;
  justify-content: space-between;
  padding: 0 12px 0 20px;
  font-size: 16px;
  font-weight: 600;
}

.post-settings-fields {
  padding: 8px 20px 24px;
}

.post-settings-mobile {
  max-width: 1200px;
  margin: 0 auto;
  padding: 30px 40px;
}

@media (max-width: 1023px) {
  .post-settings-mobile {
    padding-inline: 16px;
  }
}

@media (max-width: 767px) {
  .editor-toolbar {
    padding-inline: 16px;
  }

  .editor-back {
    padding-left: 0;
  }

  .editor-title-input :deep(.n-input-wrapper) {
    padding-inline: 16px;
  }

  .editor-title-input :deep(.n-input__input-el) {
    font-size: 24px;
  }

}
</style>

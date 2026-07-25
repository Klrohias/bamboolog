<template>
  <div class="storage-engines-view">
    <n-card :bordered="false" :title="t('storage_engine.title')">
      <template #header-extra>
        <n-button type="primary" @click="handleAdd">
          <template #icon><n-icon :component="AddOutline" /></template>
          {{ t('storage_engine.create') }}
        </n-button>
      </template>

      <n-data-table :columns="columns" :data="engines" :loading="loading" :pagination="false" :scroll-x="960" />
    </n-card>

    <n-modal v-model:show="showModal" preset="card" :title="editingId ? t('storage_engine.edit') : t('storage_engine.create')" style="width: min(720px, calc(100vw - 32px))">
      <n-form :model="form" label-placement="top">
        <n-grid :cols="2" :x-gap="16">
          <n-form-item-gi :label="t('storage_engine.name')">
            <n-input v-model:value="form.name" />
          </n-form-item-gi>
          <n-form-item-gi :label="t('storage_engine.kind')">
            <n-select v-model:value="form.kind" :options="kindOptions" :disabled="Boolean(editingId)" />
          </n-form-item-gi>
        </n-grid>

        <n-form-item :label="t('storage_engine.comments')">
          <n-input v-model:value="form.comments" type="textarea" />
        </n-form-item>

        <template v-if="form.kind === 'local'">
          <n-form-item :label="t('storage_engine.local_root')">
            <n-input v-model:value="form.local.root" :placeholder="t('storage_engine.local_root_hint')" />
          </n-form-item>
        </template>

        <template v-else>
          <n-grid :cols="2" :x-gap="16">
            <n-form-item-gi :label="t('storage_engine.s3_bucket')">
              <n-input v-model:value="form.s3.bucket" />
            </n-form-item-gi>
            <n-form-item-gi :label="t('storage_engine.s3_region')">
              <n-input v-model:value="form.s3.region" placeholder="auto" />
            </n-form-item-gi>
            <n-form-item-gi :span="2" :label="t('storage_engine.s3_endpoint')">
              <n-input v-model:value="form.s3.endpoint_url" placeholder="https://s3.example.com" />
            </n-form-item-gi>
            <n-form-item-gi :label="t('storage_engine.s3_access_key')">
              <n-input v-model:value="form.s3.access_key_id" />
            </n-form-item-gi>
            <n-form-item-gi :label="t('storage_engine.s3_secret_key')">
              <n-input v-model:value="form.s3.secret_access_key" type="password" show-password-on="click" />
            </n-form-item-gi>
            <n-form-item-gi :span="2" :label="t('storage_engine.s3_prefix')">
              <n-input v-model:value="form.s3.prefix" />
            </n-form-item-gi>
          </n-grid>
          <n-form-item :label="t('storage_engine.s3_path_style')">
            <n-switch v-model:value="form.s3.force_path_style" />
          </n-form-item>
        </template>

        <n-space>
          <n-form-item :label="t('storage_engine.enabled')">
            <n-switch v-model:value="form.enabled" />
          </n-form-item>
          <n-form-item :label="t('storage_engine.default')">
            <n-switch v-model:value="form.is_default" :disabled="!form.enabled" />
          </n-form-item>
        </n-space>
      </n-form>

      <template #footer>
        <n-space justify="end">
          <n-button @click="showModal = false">{{ t('common.cancel') }}</n-button>
          <n-button type="primary" :loading="submitting" @click="handleSubmit">{{ t('common.save') }}</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { h, onMounted, reactive, ref } from 'vue'
import {
  NButton, NCard, NDataTable, NForm, NFormItem, NFormItemGi, NGrid, NIcon, NInput, NModal,
  NSelect, NSpace, NSwitch, useDialog, useMessage, type DataTableColumns
} from 'naive-ui'
import { AddOutline, CheckmarkOutline, CloseOutline } from '@vicons/ionicons5'
import { useI18n } from 'vue-i18n'
import { storageApi, type StorageEngine } from '@/api/storage'

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog()
const loading = ref(false)
const engines = ref<StorageEngine[]>([])
const showModal = ref(false)
const submitting = ref(false)
const editingId = ref<number | null>(null)

const form = reactive({
  name: '',
  kind: 'local' as StorageEngine['kind'],
  comments: '',
  is_default: false,
  enabled: true,
  local: { root: '' },
  s3: {
    bucket: '',
    region: '',
    endpoint_url: '',
    access_key_id: '',
    secret_access_key: '',
    prefix: '',
    force_path_style: false
  }
})

const kindOptions = [
  { label: t('storage_engine.kind_local'), value: 'local' },
  { label: t('storage_engine.kind_s3'), value: 's3' }
]

const columns: DataTableColumns<StorageEngine> = [
  { title: 'ID', key: 'id', width: 72 },
  { title: t('storage_engine.name'), key: 'name' },
  {
    title: t('storage_engine.kind'),
    key: 'kind',
    render: row => row.kind === 'local' ? t('storage_engine.kind_local') : t('storage_engine.kind_s3')
  },
  { title: t('storage_engine.comments'), key: 'comments', ellipsis: { tooltip: true } },
  {
    title: t('storage_engine.enabled'),
    key: 'enabled',
    width: 90,
    render: row => h(NIcon, { component: row.enabled ? CheckmarkOutline : CloseOutline })
  },
  {
    title: t('storage_engine.default'),
    key: 'is_default',
    width: 90,
    render: row => row.is_default ? h(NIcon, { component: CheckmarkOutline }) : null
  },
  {
    title: t('storage_engine.actions'),
    key: 'actions',
    width: 150,
    render(row) {
      return h(NSpace, null, {
        default: () => [
          h(NButton, { size: 'small', onClick: () => handleEdit(row) }, { default: () => t('storage_engine.edit') }),
          h(NButton, { size: 'small', type: 'error', onClick: () => handleDelete(row) }, { default: () => t('storage_engine.delete') })
        ]
      })
    }
  }
]

function resetForm() {
  form.name = ''
  form.kind = 'local'
  form.comments = ''
  form.is_default = false
  form.enabled = true
  form.local.root = ''
  form.s3.bucket = ''
  form.s3.region = ''
  form.s3.endpoint_url = ''
  form.s3.access_key_id = ''
  form.s3.secret_access_key = ''
  form.s3.prefix = ''
  form.s3.force_path_style = false
}

function applyConfig(row: StorageEngine) {
  resetForm()
  form.name = row.name
  form.kind = row.kind
  form.comments = row.comments
  form.is_default = row.is_default
  form.enabled = row.enabled

  if (!row.config_json) return
  try {
    const config = JSON.parse(row.config_json)
    if (row.kind === 'local') {
      form.local.root = config.root ?? ''
    } else {
      form.s3.bucket = config.bucket ?? ''
      form.s3.region = config.region ?? ''
      form.s3.endpoint_url = config.endpoint_url ?? ''
      form.s3.access_key_id = config.access_key_id ?? ''
      form.s3.secret_access_key = config.secret_access_key ?? ''
      form.s3.prefix = config.prefix ?? ''
      form.s3.force_path_style = config.force_path_style ?? false
    }
  } catch {
    message.warning(t('storage_engine.invalid_config'))
  }
}

function buildConfig(): string | undefined {
  if (form.kind === 'local') {
    return JSON.stringify({ root: form.local.root.trim() || undefined })
  }

  const config = {
    bucket: form.s3.bucket.trim(),
    region: form.s3.region.trim() || undefined,
    endpoint_url: form.s3.endpoint_url.trim() || undefined,
    access_key_id: form.s3.access_key_id.trim() || undefined,
    secret_access_key: form.s3.secret_access_key || undefined,
    prefix: form.s3.prefix.trim() || undefined,
    force_path_style: form.s3.force_path_style
  }
  return JSON.stringify(config)
}

async function fetchEngines() {
  loading.value = true
  try {
    const { data } = await storageApi.list()
    engines.value = data.data
  } catch {
    message.error(t('storage_engine.fetch_failed'))
  } finally {
    loading.value = false
  }
}

function handleAdd() {
  editingId.value = null
  resetForm()
  showModal.value = true
}

function handleEdit(row: StorageEngine) {
  editingId.value = row.id
  applyConfig(row)
  showModal.value = true
}

async function handleSubmit() {
  if (!form.name.trim()) {
    message.error(t('storage_engine.name_required'))
    return
  }

  submitting.value = true
  const payload = {
    name: form.name.trim(),
    kind: form.kind,
    comments: form.comments,
    config_json: buildConfig(),
    is_default: form.is_default,
    enabled: form.enabled
  }
  try {
    if (editingId.value) {
      await storageApi.update(editingId.value, payload)
      message.success(t('storage_engine.update_success'))
    } else {
      await storageApi.create(payload)
      message.success(t('storage_engine.create_success'))
    }
    showModal.value = false
    await fetchEngines()
  } catch {
    message.error(t('storage_engine.operation_failed'))
  } finally {
    submitting.value = false
  }
}

function handleDelete(row: StorageEngine) {
  dialog.warning({
    title: t('storage_engine.delete'),
    content: t('storage_engine.delete_confirm', { name: row.name }),
    positiveText: t('common.delete'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      try {
        await storageApi.delete(row.id)
        message.success(t('storage_engine.delete_success'))
        await fetchEngines()
      } catch {
        message.error(t('storage_engine.delete_failed'))
      }
    }
  })
}

onMounted(fetchEngines)
</script>

<template>
  <div class="storage-engines-view">
    <n-card :bordered="false" :title="$t('storage_engine.title')">
      <template #header-extra>
        <n-button type="primary" @click="handleAdd">{{ $t('storage_engine.create') }}</n-button>
      </template>
      <n-data-table :columns="columns" :data="engines" :loading="loading" :pagination="false" />
    </n-card>

    <n-modal v-model:show="showModal" preset="card" :title="editingId ? 'Edit Engine' : 'New Engine'">
      <n-form :model="form" label-placement="left" label-width="80">
        <n-form-item label="Name" path="name">
          <n-input v-model:value="form.name" placeholder="Engine Name" />
        </n-form-item>
        <n-form-item label="Type" path="type">
          <n-select v-model:value="form.type" :options="typeOptions" />
        </n-form-item>
        <n-form-item label="Comments" path="comments">
          <n-input v-model:value="form.comments" type="textarea" placeholder="Comments" />
        </n-form-item>
        <n-form-item label="Config" path="config">
          <n-input v-model:value="form.config" type="textarea" placeholder="JSON Configuration (Optional)" />
        </n-form-item>
      </n-form>
      <template #footer>
        <n-space justify="end">
          <n-button @click="showModal = false">Cancel</n-button>
          <n-button type="primary" :loading="submitting" @click="handleSubmit">Save</n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { h, ref, onMounted, reactive } from 'vue'
import {
  NButton, NCard, NDataTable, NSpace, NModal, NForm, NFormItem, NInput, NSelect, useMessage, useDialog
} from 'naive-ui'
import type { DataTableColumns } from 'naive-ui'
import { storageApi, type StorageEngine } from '@/api/storage'
import { useI18n } from 'vue-i18n'

const message = useMessage()
const dialog = useDialog()
const loading = ref(false)
const engines = ref<StorageEngine[]>([])
const showModal = ref(false)
const submitting = ref(false)
const editingId = ref<number | null>(null)
const { t } = useI18n()

const form = reactive({
  name: '',
  type: 'internal',
  comments: '',
  config: ''
})

const typeOptions = [
  { label: 'Internal', value: 'internal' },
  { label: 'S3', value: 's3' }
]

const columns: DataTableColumns<StorageEngine> = [
  { title: 'ID', key: 'id', width: 60 },
  { title: t('storage_engine.name'), key: 'name' },
  { title: t('storage_engine.type'), key: 'type' },
  { title: t('storage_engine.comments'), key: 'comments' },
  {
    title: t('storage_engine.actions'),
    key: 'actions',
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

async function fetchEngines() {
  loading.value = true
  try {
    const { data } = await storageApi.list()
    engines.value = data.data
  } catch (e: any) {
    message.error('Failed to fetch storage engines')
  } finally {
    loading.value = false
  }
}

function handleAdd() {
  editingId.value = null
  form.name = ''
  form.type = 'internal'
  form.comments = ''
  form.config = ''
  showModal.value = true
}

function handleEdit(row: StorageEngine) {
  editingId.value = row.id
  form.name = row.name
  form.type = row.type
  form.comments = row.comments
  form.config = row.config || ''
  showModal.value = true
}

async function handleSubmit() {
  submitting.value = true
  try {
    const payload = {
      name: form.name,
      type: form.type as 'internal' | 's3',
      comments: form.comments,
      config: form.config || undefined
    }
    if (editingId.value) {
      await storageApi.update(editingId.value, payload)
      message.success('Engine updated')
    } else {
      await storageApi.create(payload)
      message.success('Engine created')
    }
    showModal.value = false
    fetchEngines()
  } catch (e: any) {
    message.error('Operation failed')
  } finally {
    submitting.value = false
  }
}

function handleDelete(row: StorageEngine) {
  dialog.warning({
    title: 'Delete Engine',
    content: `Are you sure you want to delete ${row.name}?`,
    positiveText: 'Delete',
    negativeText: 'Cancel',
    onPositiveClick: async () => {
      try {
        await storageApi.delete(row.id)
        message.success('Engine deleted')
        fetchEngines()
      } catch (e) {
        message.error('Delete failed')
      }
    }
  })
}

onMounted(fetchEngines)
</script>

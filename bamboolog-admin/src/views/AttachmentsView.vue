<template>
  <div class="attachments-view">
    <n-card :bordered="false">
      <template #header>
        <n-space justify="space-between">
          <span>{{ t('attachments.title') }}</span>
          <n-space>
            <n-input v-model:value="filters.mime" :placeholder="t('attachments.filter_mime')" clearable
              @keydown.enter="fetchAttachments(1)">
              <template #prefix>
                <n-icon :component="SearchOutline" />
              </template>
            </n-input>
            <n-select v-model:value="filters.storage_engine_id" :options="engineOptions" clearable
              :placeholder="t('attachments.filter_storage')" style="width: 160px" />
            <n-button @click="handleReset">
              <template #icon>
                <n-icon><refresh-outline /></n-icon>
              </template>
            </n-button>
            <n-button type="primary" @click="showUploadModal = true">
              <template #icon>
                <n-icon><cloud-upload-outline /></n-icon>
              </template>
              {{ t('attachments.upload') }}
            </n-button>
          </n-space>
        </n-space>
      </template>

      <n-data-table remote :columns="columns" :data="attachments" :loading="loading" :pagination="pagination"
        @update:page="handlePageChange" @update:sorter="handleSorterChange" />
    </n-card>

    <n-modal v-model:show="showUploadModal" preset="card" :title="t('attachments.upload_modal_title')"
      style="width: 500px">
      <n-space vertical size="large">
        <n-form-item :label="t('attachments.select_storage')">
          <n-select v-model:value="uploadEngineId" :options="engineOptions"
            :placeholder="t('attachments.select_storage')" />
        </n-form-item>

        <n-upload multiple :custom-request="handleUpload" :show-file-list="true" trigger="drag"
          :disabled="!uploadEngineId">
          <n-upload-dragger>
            <div style="margin-bottom: 12px">
              <n-icon size="48" :depth="3">
                <cloud-upload-outline />
              </n-icon>
            </div>
            <n-text style="font-size: 16px">
              {{ t('attachments.drag_or_click') }}
            </n-text>
          </n-upload-dragger>
        </n-upload>
      </n-space>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { h, ref, onMounted, computed, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  NButton,
  NIcon,
  NSpace,
  NCard,
  NDataTable,
  NUpload,
  NUploadDragger,
  NModal,
  NSelect,
  NInput,
  NFormItem,
  NText,
  useMessage,
  useDialog,
  type DataTableColumns,
  type UploadCustomRequestOptions
} from 'naive-ui'
import { CloudUploadOutline, TrashOutline, CopyOutline, SearchOutline, RefreshOutline } from '@vicons/ionicons5'
import { attachmentApi, type Attachment } from '@/api/attachments'
import { storageApi, type StorageEngine } from '@/api/storage'

const { t } = useI18n()
const message = useMessage()
const dialog = useDialog()

const loading = ref(false)
const attachments = ref<Attachment[]>([])
const engines = ref<StorageEngine[]>([])

// Upload Modal State
const showUploadModal = ref(false)
const uploadEngineId = ref<number | null>(null)

// Filtering and Sorting
const filters = reactive({
  mime: '',
  storage_engine_id: null as number | null
})

const pagination = ref({
  page: 1,
  pageSize: 20,
  itemCount: 0,
  pageCount: 1
})

const engineOptions = computed(() => engines.value.map(e => ({ label: e.name, value: e.id })))

const columns: DataTableColumns<Attachment> = [
  {
    title: 'ID',
    key: 'id',
    width: 60,
    sorter: true
  },
  {
    title: t('attachments.preview'),
    key: 'preview',
    render(row) {
      if (row.mime.startsWith('image/')) {
        return h('img', {
          src: `/attachments/${row.hash}`,
          style: 'max-height: 50px; max-width: 100px; object-fit: cover; border-radius: 4px; cursor: pointer;',
          onClick: () => window.open(`/attachments/${row.hash}`, '_blank')
        })
      }
      return row.mime
    }
  },
  {
    title: t('attachments.filter_mime'),
    key: 'mime',
    filter: true,
    filterOptionValue: filters.mime,
  },
  {
    title: 'Hash',
    key: 'hash',
    ellipsis: {
      tooltip: true
    }
  },
  {
    title: t('attachments.created_at'),
    key: 'created_at',
    sorter: true,
    render(row) {
      return new Date(row.created_at).toLocaleString()
    }
  },
  {
    title: t('attachments.actions'),
    key: 'actions',
    render(row) {
      return h(NSpace, null, {
        default: () => [
          h(
            NButton,
            {
              size: 'small',
              quaternary: true,
              onClick: () => copyLink(row)
            },
            { icon: () => h(NIcon, null, { default: () => h(CopyOutline) }) }
          ),
          h(
            NButton,
            {
              size: 'small',
              quaternary: true,
              type: 'error',
              onClick: () => handleDelete(row)
            },
            { icon: () => h(NIcon, null, { default: () => h(TrashOutline) }) }
          )
        ]
      })
    }
  }
]

async function fetchEngines() {
  try {
    const { data } = await storageApi.list()
    engines.value = data.data
    if (engines.value.length > 0) {
      const internal = engines.value.find(e => e.type === 'internal')
      uploadEngineId.value = internal ? internal.id : (engines.value[0]?.id ?? null)
    }
  } catch (e) {
    message.warning('Failed to fetch storage engines')
  }
}

const sortState = reactive({
  columnKey: null as string | null,
  order: null as 'asc' | 'desc' | null
})

async function fetchAttachments(page = 1) {
  loading.value = true
  try {
    const { data } = await attachmentApi.list(
      page,
      pagination.value.pageSize,
      filters.mime || undefined,
      filters.storage_engine_id || undefined,
      sortState.columnKey || undefined,
      sortState.order || undefined
    )
    attachments.value = data.data.items
    pagination.value.page = data.data.page
    pagination.value.pageSize = data.data.size
    pagination.value.itemCount = data.data.total
    pagination.value.pageCount = data.data.total_pages
  } catch (error) {
    message.error(t('attachments.fetch_failed'))
  } finally {
    loading.value = false
  }
}

async function handleUpload({ file, onFinish, onError }: UploadCustomRequestOptions) {
  if (!file.file || !uploadEngineId.value) return

  try {
    await attachmentApi.upload(file.file, uploadEngineId.value)
    message.success(t('attachments.upload_success'))
    onFinish()
    showUploadModal.value = false
    fetchAttachments(pagination.value.page)
  } catch (error) {
    message.error(t('attachments.upload_failed'))
    onError()
  }
}

function handleDelete(row: Attachment) {
  dialog.warning({
    title: t('common.confirm'),
    content: t('common.confirm_delete'),
    positiveText: t('common.delete'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      try {
        await attachmentApi.delete(row.id)
        message.success(t('attachments.delete_success'))
        fetchAttachments(pagination.value.page)
      } catch (error) {
        message.error(t('attachments.delete_failed'))
      }
    }
  })
}

function copyLink(row: Attachment) {
  const url = `${window.location.origin}/attachments/${row.hash}`
  navigator.clipboard.writeText(url).then(() => {
    message.success(t('attachments.copy_success'))
  })
}

function handlePageChange(page: number) {
  fetchAttachments(page)
}

function handleSorterChange(sorter: { columnKey: string, order: 'ascend' | 'descend' | false }) {
  if (!sorter || sorter.order === false) {
    sortState.columnKey = null
    sortState.order = null
  } else {
    sortState.columnKey = sorter.columnKey
    sortState.order = sorter.order === 'ascend' ? 'asc' : 'desc'
  }
  fetchAttachments(1)
}

function handleReset() {
  filters.mime = ''
  filters.storage_engine_id = null
  sortState.columnKey = null
  sortState.order = null
  fetchAttachments(1)
}

// Watch filters to refresh
watch([() => filters.storage_engine_id], () => {
  fetchAttachments(1)
})

onMounted(() => {
  fetchEngines()
  fetchAttachments()
})
</script>

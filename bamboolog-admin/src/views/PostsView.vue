<template>
  <n-space vertical size="large">
    <n-page-header :title="$t('common.posts')">
      <template #extra>
        <n-button type="primary" @click="$router.push('/posts/new')">{{ $t('posts.new_post') }}</n-button>
      </template>
    </n-page-header>

    <n-card>
      <n-space vertical>
        <n-space>
          <n-input v-model:value="filters.title" :placeholder="$t('posts.title')" clearable @update:value="handleSearch" />
          <n-input v-model:value="filters.name" :placeholder="$t('posts.slug')" clearable @update:value="handleSearch" />
          <n-button @click="fetchPosts">{{ $t('common.search') }}</n-button>
        </n-space>
        <n-data-table
          remote
          :columns="columns"
          :data="posts"
          :loading="loading"
          :pagination="pagination"
          @update:page="handlePageChange"
          @update:page-size="handlePageSizeChange"
          @update:sorter="handleSorterChange"
        />
      </n-space>
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { h, onMounted, ref, reactive, computed } from 'vue'
import { NButton, NSpace, NInput, useMessage, useDialog, type DataTableColumns } from 'naive-ui'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import api from '@/api'

const { t } = useI18n()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()
const loading = ref(false)
const posts = ref([])

const filters = reactive({
  title: '',
  name: ''
})

const sorter = reactive({
  columnKey: 'id',
  order: 'descend'
})

const pagination = reactive({
  page: 1,
  pageSize: 10,
  itemCount: 0,
  showSizePicker: true,
  pageSizes: [10, 20, 50]
})

const columns = computed<DataTableColumns<any>>(() => [
  { title: 'ID', key: 'id', width: 80, sorter: true, sortOrder: sorter.columnKey === 'id' ? sorter.order : false },
  { title: t('posts.title'), key: 'title', sorter: true, sortOrder: sorter.columnKey === 'title' ? sorter.order : false },
  { title: t('posts.slug'), key: 'name', sorter: true, sortOrder: sorter.columnKey === 'name' ? sorter.order : false },
  {
    title: t('posts.created_at'),
    key: 'created_at',
    sorter: true,
    sortOrder: sorter.columnKey === 'created_at' ? sorter.order : false,
    render(row) {
      return new Date(row.created_at).toLocaleString()
    }
  },
  {
    title: t('posts.actions'),
    key: 'actions',
    render(row) {
      return h(
        NSpace,
        {},
        {
          default: () => [
            h(
              NButton,
              {
                size: 'small',
                onClick: () => router.push(`/posts/edit/${row.id}`)
              },
              { default: () => t('common.edit') }
            ),
            h(
              NButton,
              {
                size: 'small',
                type: 'error',
                onClick: () => handleDelete(row.id)
              },
              { default: () => t('common.delete') }
            )
          ]
        }
      )
    }
  }
])

async function fetchPosts() {
  loading.value = true
  try {
    const params: any = {
      page: pagination.page,
      page_size: pagination.pageSize,
      sort_by: sorter.columnKey,
      order: sorter.order === 'ascend' ? 'asc' : 'desc'
    }
    if (filters.title) params.title = filters.title
    if (filters.name) params.name = filters.name

    const { data } = await api.get('/posts/', { params })
    posts.value = data.data.posts
    pagination.itemCount = data.data.total
  } catch (e: any) {
    message.error(e.response?.data?.message || t('posts.fetch_failed'))
  } finally {
    loading.value = false
  }
}

function handleSearch() {
  pagination.page = 1
  fetchPosts()
}

function handlePageChange(page: number) {
  pagination.page = page
  fetchPosts()
}

function handlePageSizeChange(pageSize: number) {
  pagination.pageSize = pageSize
  pagination.page = 1
  fetchPosts()
}

function handleSorterChange(s: { columnKey: string, order: 'ascend' | 'descend' | false }) {
  if (!s.order) {
    sorter.columnKey = 'id'
    sorter.order = 'descend'
  } else {
    sorter.columnKey = s.columnKey
    sorter.order = s.order
  }
  fetchPosts()
}

async function handleDelete(id: number) {
  dialog.warning({
    title: t('common.delete'),
    content: t('common.confirm_delete'),
    positiveText: t('common.confirm'),
    negativeText: t('common.cancel'),
    onPositiveClick: async () => {
      try {
        await api.delete(`/posts/${id}`)
        message.success(t('posts.delete_success'))
        fetchPosts()
      } catch (e: any) {
        message.error(e.response?.data?.message || t('common.error'))
      }
    }
  })
}

onMounted(fetchPosts)
</script>


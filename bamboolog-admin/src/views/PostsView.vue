<template>
  <n-space vertical size="large">
    <n-page-header :title="$t('common.posts')">
      <template #extra>
        <n-button type="primary" @click="$router.push('/posts/new')">{{ $t('posts.new_post') }}</n-button>
      </template>
    </n-page-header>

    <n-card>
      <n-data-table
        :columns="columns"
        :data="posts"
        :loading="loading"
        :pagination="pagination"
      />
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { h, onMounted, ref, reactive, computed } from 'vue'
import { NButton, NSpace, useMessage, useDialog, type DataTableColumns } from 'naive-ui'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import api from '@/api'

const { t } = useI18n()
const router = useRouter()
const message = useMessage()
const dialog = useDialog()
const loading = ref(false)
const posts = ref([])

const pagination = reactive({
  page: 1,
  pageSize: 10,
  showSizePicker: true,
  pageSizes: [10, 20, 50],
  onChange: (page: number) => {
    pagination.page = page
  },
  onUpdatePageSize: (pageSize: number) => {
    pagination.pageSize = pageSize
    pagination.page = 1
  }
})

const columns = computed<DataTableColumns<any>>(() => [
  { title: 'ID', key: 'id', width: 80 },
  { title: t('posts.title'), key: 'title' },
  { title: t('posts.slug'), key: 'name' },
  {
    title: t('posts.created_at'),
    key: 'created_at',
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
    const { data } = await api.get('/posts/')
    posts.value = data.data
  } catch (e: any) {
    message.error(e.response?.data?.message || t('posts.fetch_failed'))
  } finally {
    loading.value = false
  }
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


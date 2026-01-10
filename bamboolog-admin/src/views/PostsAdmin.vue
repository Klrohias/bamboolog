<template>
  <n-layout>
    <n-layout-header style="display:flex;align-items:center;justify-content:space-between">
      <div style="font-weight:bold">Bamboolog Admin</div>
      <div>
        <n-button @click="logout" size="small">Logout</n-button>
      </div>
    </n-layout-header>

    <n-layout-content style="padding:16px">
      <n-card title="Create Post">
        <n-form>
          <n-form-item label="Title">
            <n-input v-model:value="newPost.title" />
          </n-form-item>
          <n-form-item label="Name (slug)">
            <n-input v-model:value="newPost.name" />
          </n-form-item>
          <n-form-item label="Content">
            <n-input type="textarea" rows="8" v-model:value="newPost.content" />
          </n-form-item>
          <n-form-item>
            <n-button type="primary" @click="createPost">Create</n-button>
          </n-form-item>
        </n-form>
      </n-card>

      <n-card title="Fetch / Edit / Delete" style="margin-top:16px">
        <n-space vertical>
          <n-input v-model:value="fetchId" placeholder="Post ID" style="width:200px" />
          <n-button @click="fetchPost">Fetch</n-button>

          <div v-if="post"> 
            <h3>{{ post.title }} (ID: {{ post.id }})</h3>
            <n-button @click="fetchRendered">View Rendered</n-button>
            <n-button type="error" @click="deletePost">Delete</n-button>

            <n-divider />

            <n-form>
              <n-form-item label="Title">
                <n-input v-model:value="edit.title" />
              </n-form-item>
              <n-form-item label="Name">
                <n-input v-model:value="edit.name" />
              </n-form-item>
              <n-form-item label="Content">
                <n-input type="textarea" rows="8" v-model:value="edit.content" />
              </n-form-item>
              <n-form-item>
                <n-button type="primary" @click="editPost">Save</n-button>
              </n-form-item>
            </n-form>
          </div>

          <div v-if="renderedHtml" v-html="renderedHtml" style="border:1px solid #eee;padding:12px"></div>
        </n-space>
      </n-card>
    </n-layout-content>
  </n-layout>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import api, { setAuthToken } from '../api'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'

const router = useRouter()
const msg = useMessage()

const newPost = reactive({ title: '', name: '', content: '' })
const fetchId = ref('')
const post: any = ref(null)
const edit = reactive({ title: '', name: '', content: '' })
const renderedHtml = ref('')

async function createPost() {
  try {
    await api.put('/posts/', {
      title: newPost.title,
      name: newPost.name,
      content: newPost.content,
    })
    msg.success('Created')
    newPost.title = ''
    newPost.name = ''
    newPost.content = ''
  } catch (e: any) {
    msg.error(e?.response?.data?.message || 'Create failed')
  }
}

async function fetchPost() {
  if (!fetchId.value) return
  try {
    const { data } = await api.get(`/posts/${fetchId.value}`)
    post.value = data.data
    edit.title = post.value.title
    edit.name = post.value.name
    edit.content = post.value.content
    renderedHtml.value = ''
  } catch (e: any) {
    msg.error(e?.response?.data?.message || 'Fetch failed')
  }
}

async function fetchRendered() {
  if (!fetchId.value) return
  try {
    const { data } = await api.get(`/posts/${fetchId.value}/rendered`)
    // backend returns HTML in data.data
    renderedHtml.value = data.data
  } catch (e: any) {
    msg.error('Render failed')
  }
}

async function editPost() {
  if (!fetchId.value) return
  try {
    await api.post(`/posts/${fetchId.value}`, {
      title: edit.title,
      name: edit.name,
      content: edit.content,
    })
    msg.success('Saved')
    fetchPost()
  } catch (e: any) {
    msg.error('Save failed')
  }
}

async function deletePost() {
  if (!fetchId.value) return
  try {
    await api.delete(`/posts/${fetchId.value}`)
    msg.success('Deleted')
    post.value = null
    renderedHtml.value = ''
  } catch (e: any) {
    msg.error('Delete failed')
  }
}

function logout() {
  setAuthToken(null)
  router.push('/login')
}

</script>

<style scoped>
/* simple spacing */
.n-layout-content { max-width: 900px; margin: 24px auto }

</style>

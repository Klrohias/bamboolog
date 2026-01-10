<template>
  <n-space vertical align="center">
    <n-card title="Admin Login" style="width: 420px;">
      <n-form :model="form" @submit.prevent="doLogin">
        <n-form-item label="Username">
          <n-input v-model:value="form.username" placeholder="admin" />
        </n-form-item>

        <n-form-item label="Password">
          <n-input type="password" v-model:value="form.password" />
        </n-form-item>

        <n-form-item>
          <n-button type="primary" @click="doLogin">Login</n-button>
        </n-form-item>
      </n-form>
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useRouter } from 'vue-router'
import { NMessageProvider, useMessage } from 'naive-ui'
import api, { setAuthToken } from '../api'

const router = useRouter()
const msg = useMessage()

const form = reactive({ username: '', password: '' })

async function doLogin() {
  try {
    const { data } = await api.post('/user/auth', {
      username: form.username,
      password: form.password,
    })
    const token = data.data.token
    setAuthToken(token)
    msg.success('Logged in')
    router.push('/posts')
  } catch (e: any) {
    msg.error(e?.response?.data?.message || 'Login failed')
  }
}
</script>

<style scoped>
:root{--center: display:flex;}
div{display:flex;justify-content:center;margin-top:40px}
</style>

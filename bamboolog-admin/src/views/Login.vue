<template>
  <div class="login-container">
    <n-card class="login-card" title="Bamboolog Admin" size="huge" :bordered="false">
      <div class="login-logo">🎋</div>
      <n-form :model="form" @submit.prevent="doLogin">
        <n-form-item :label="$t('login.username')">
          <n-input
            v-model:value="form.username"
            :placeholder="$t('login.username')"
            @keyup.enter="doLogin"
          >
            <template #prefix>
              <n-icon :component="PersonOutline" />
            </template>
          </n-input>
        </n-form-item>

        <n-form-item :label="$t('login.password')">
          <n-input
            type="password"
            show-password-on="mousedown"
            v-model:value="form.password"
            :placeholder="$t('login.password')"
            @keyup.enter="doLogin"
          >
            <template #prefix>
              <n-icon :component="LockClosedOutline" />
            </template>
          </n-input>
        </n-form-item>

        <n-button
          type="primary"
          block
          :loading="loading"
          @click="doLogin"
        >
          {{ $t('login.submit') }}
        </n-button>
      </n-form>
      <div class="login-footer">
        <n-space justify="center">
          <n-button quaternary circle @click="settingsStore.toggleTheme">
            <template #icon>
              <n-icon v-if="settingsStore.theme === 'dark'"><sunny-outline /></n-icon>
              <n-icon v-else><moon-outline /></n-icon>
            </template>
          </n-button>
          <n-button quaternary circle @click="handleToggleLocale">
            <template #icon>
              <n-icon><language-outline /></n-icon>
            </template>
          </n-button>
        </n-space>
      </div>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { PersonOutline, LockClosedOutline, SunnyOutline, MoonOutline, LanguageOutline } from '@vicons/ionicons5'
import api, { setAuthToken } from '../api'
import { useSettingsStore } from '../stores/settings'

const { t, locale } = useI18n()
const settingsStore = useSettingsStore()
const router = useRouter()
const msg = useMessage()
const loading = ref(false)

const form = reactive({ username: '', password: '' })

async function doLogin() {
  if (!form.username || !form.password) {
    msg.warning(t('login.username') + ' / ' + t('login.password') + ' ?')
    return
  }
  loading.value = true
  try {
    const { data } = await api.post('/user/auth', {
      username: form.username,
      password: form.password,
    })
    const token = data.data.token
    setAuthToken(token)
    msg.success(t('common.login') + ' ' + t('common.success'))
    router.push('/posts')
  } catch (e: any) {
    msg.error(e?.response?.data?.message || t('common.error'))
  } finally {
    loading.value = false
  }
}

function handleToggleLocale() {
  const newLocale = settingsStore.locale === 'zh-CN' ? 'en-US' : 'zh-CN'
  settingsStore.locale = newLocale
  locale.value = newLocale
}
</script>

<style scoped>
.login-container {
  height: 100vh;
  width: 100vw;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--n-color);
}

.login-card {
  width: 100%;
  max-width: 400px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
  border-radius: 8px;
}

.login-logo {
  font-size: 40px;
  text-align: center;
  margin-bottom: 20px;
}

.login-footer {
  margin-top: 24px;
}

:deep(.n-card-header__main) {
  text-align: center;
  font-weight: 600;
}
</style>


<template>
  <n-space vertical size="large">
    <n-page-header :title="$t('common.settings')"></n-page-header>

    <n-card :title="$t('settings.site_settings')">
      <n-form :model="settings.site">
        <n-form-item :label="$t('settings.site_name')">
          <n-input v-model:value="settings.site.site_name" />
        </n-form-item>
        <n-form-item :label="$t('settings.base_url')">
          <n-input v-model:value="settings.site.base_url" />
        </n-form-item>
        <n-button type="primary" @click="saveSettings('site')">{{ $t('common.save') }}</n-button>
      </n-form>
    </n-card>

    <n-card :title="$t('settings.theme_settings')" style="margin-top: 24px">
      <n-form :model="settings.theme">
        <n-form-item :label="$t('settings.current_theme')">
          <n-select v-model:value="settings.theme.current" :options="themeOptions" />
        </n-form-item>
        <n-button type="primary" @click="saveSettings('theme')">{{ $t('common.save') }}</n-button>
      </n-form>
    </n-card>
  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import api from '../api'

const { t } = useI18n()
const message = useMessage()
const settings = ref({
  site: {
    site_name: '',
    base_url: ''
  },
  theme: {
    current: ''
  }
})

const themeOptions = ref<{label: string, value: string}[]>([])

async function fetchSettings() {
  try {
    const { data } = await api.get('/settings/')
    settings.value = data.data
  } catch (e) {
    message.error(t('settings.fetch_failed'))
  }
}

async function fetchThemes() {
  try {
    const { data } = await api.get('/settings/themes')
    themeOptions.value = data.data.map((t: string) => ({ label: t, value: t }))
  } catch (e) {
    message.error(t('common.error'))
  }
}

async function saveSettings(type: 'site' | 'theme') {
  try {
    const payload = type === 'site' ? { site: settings.value.site } : { theme: settings.value.theme }
    await api.post('/settings/', payload)
    message.success(t('settings.save_success'))
  } catch (e) {
    message.error(t('common.error'))
  }
}

onMounted(() => {
  fetchSettings()
  fetchThemes()
})
</script>


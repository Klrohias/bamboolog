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
        <n-form-item :label="$t('settings.language')">
          <n-input v-model:value="settings.site.language" placeholder="en" />
        </n-form-item>
        <n-form-item :label="$t('settings.favicon_url')">
          <n-input v-model:value="settings.site.favicon_url" :placeholder="$t('settings.optional_url_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.site_description')">
          <n-input v-model:value="settings.site.description" type="textarea" :autosize="{ minRows: 2, maxRows: 4 }" :placeholder="$t('settings.site_description_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.copyright')">
          <n-input v-model:value="settings.site.copyright" :placeholder="$t('settings.copyright_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.navigation')">
          <n-space vertical style="width: 100%">
            <n-space v-for="(item, index) in settings.site.navigation" :key="index" align="center" style="width: 100%" :wrap="false">
              <n-input v-model:value="item.label" :placeholder="$t('settings.navigation_label')" />
              <n-input v-model:value="item.url" :placeholder="$t('settings.navigation_url')" />
              <n-button type="error" secondary @click="removeNavigation(index)">{{ $t('common.delete') }}</n-button>
            </n-space>
            <n-button dashed @click="addNavigation">{{ $t('settings.add_navigation') }}</n-button>
          </n-space>
        </n-form-item>
        <n-form-item :label="$t('settings.posts_per_page')">
          <n-input-number v-model:value="settings.site.posts_per_page" :min="1" :max="100" style="width: 100%" />
        </n-form-item>
        <n-button type="primary" @click="saveSettings">{{ $t('common.save') }}</n-button>
      </n-form>
    </n-card>

  </n-space>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { settingsApi } from '@/api/settings'

const { t } = useI18n()
const message = useMessage()
const settings = ref({
  site: {
    site_name: '',
    base_url: '',
    language: 'en',
    favicon_url: '',
    description: '',
    copyright: '',
    navigation: [] as Array<{ label: string; url: string }>,
    posts_per_page: 10
  }
})

async function fetchSettings() {
  try {
    const { data } = await settingsApi.get()
    settings.value = data.data as any
    settings.value.site.navigation ||= []
    settings.value.site.language ||= 'en'
    settings.value.site.favicon_url ||= ''
    settings.value.site.posts_per_page ||= 10
  } catch (e) {
    message.error(t('settings.fetch_failed'))
  }
}

function addNavigation() {
  settings.value.site.navigation.push({ label: '', url: '' })
}

function removeNavigation(index: number) {
  settings.value.site.navigation.splice(index, 1)
}

async function saveSettings() {
  try {
    await settingsApi.update({ site: settings.value.site })
    message.success(t('settings.save_success'))
  } catch (e) {
    message.error(t('common.error'))
  }
}

onMounted(() => {
  fetchSettings()
})
</script>

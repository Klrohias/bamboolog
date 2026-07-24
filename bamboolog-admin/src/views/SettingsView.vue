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
        <n-form-item :label="$t('settings.manifest_url')">
          <n-input v-model:value="settings.site.manifest_url" :placeholder="$t('settings.optional_url_placeholder')" />
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
        <n-form-item :label="$t('settings.comment_provider')">
          <n-select v-model:value="settings.site.comments.provider" :options="commentProviderOptions" />
        </n-form-item>
        <n-form-item v-if="settings.site.comments.provider !== 'disabled'" :label="$t('settings.comment_config')">
          <n-input v-model:value="commentsConfigJson" type="textarea" :autosize="{ minRows: 5, maxRows: 12 }" :placeholder="$t('settings.comment_config_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.google_cse_id')">
          <n-input v-model:value="settings.site.search.google_cse_id" :placeholder="$t('settings.optional_integration_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.google_analytics_id')">
          <n-input v-model:value="settings.site.analytics.google_analytics_id" :placeholder="$t('settings.optional_integration_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.clarity_project_id')">
          <n-input v-model:value="settings.site.analytics.clarity_project_id" :placeholder="$t('settings.optional_integration_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.cloudflare_beacon_token')">
          <n-input v-model:value="settings.site.analytics.cloudflare_beacon_token" :placeholder="$t('settings.optional_integration_placeholder')" />
        </n-form-item>
        <n-form-item :label="$t('settings.head_html')">
          <n-input v-model:value="settings.site.head_html" type="textarea" :autosize="{ minRows: 3, maxRows: 10 }" :placeholder="$t('settings.head_html_placeholder')" />
        </n-form-item>
        <n-button type="primary" @click="saveSettings">{{ $t('common.save') }}</n-button>
      </n-form>
    </n-card>

  </n-space>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from 'vue'
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
    manifest_url: '',
    description: '',
    copyright: '',
    navigation: [] as Array<{ label: string; url: string }>,
    comments: { provider: 'disabled', config: {} as Record<string, string> },
    search: { google_cse_id: '' },
    analytics: { google_analytics_id: '', clarity_project_id: '', cloudflare_beacon_token: '' },
    head_html: ''
  }
})
const commentsConfigJson = ref('{}')
const commentProviderOptions = computed(() => [
  { label: t('settings.comment_provider_disabled'), value: 'disabled' },
  { label: 'Disqus', value: 'disqus' },
  { label: 'Utterances', value: 'utterances' },
  { label: 'Giscus', value: 'giscus' },
  { label: 'LiveRe', value: 'livere' },
  { label: 'Twikoo', value: 'twikoo' },
  { label: 'Waline', value: 'waline' }
])

async function fetchSettings() {
  try {
    const { data } = await settingsApi.get()
    settings.value = data.data as any
    settings.value.site.navigation ||= []
    settings.value.site.language ||= 'en'
    settings.value.site.favicon_url ||= ''
    settings.value.site.manifest_url ||= ''
    settings.value.site.comments ||= { provider: 'disabled', config: {} }
    settings.value.site.comments.provider ||= 'disabled'
    settings.value.site.comments.config ||= {}
    settings.value.site.search ||= { google_cse_id: '' }
    settings.value.site.search.google_cse_id ||= ''
    settings.value.site.analytics ||= { google_analytics_id: '', clarity_project_id: '', cloudflare_beacon_token: '' }
    settings.value.site.analytics.google_analytics_id ||= ''
    settings.value.site.analytics.clarity_project_id ||= ''
    settings.value.site.analytics.cloudflare_beacon_token ||= ''
    settings.value.site.head_html ||= ''
    commentsConfigJson.value = JSON.stringify(settings.value.site.comments.config, null, 2)
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
    const config = JSON.parse(commentsConfigJson.value)
    if (typeof config !== 'object' || config === null || Array.isArray(config) || Object.values(config).some(value => typeof value !== 'string')) {
      throw new Error('Invalid comment configuration')
    }
    settings.value.site.comments.config = config
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

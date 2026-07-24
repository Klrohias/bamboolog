<template>
  <n-space vertical size="large">
    <n-page-header :title="$t('themes.title')" :subtitle="$t('themes.subtitle')" />

    <n-spin :show="loading">
      <n-grid v-if="themes.length" cols="1 s:2 l:3" :x-gap="16" :y-gap="16" responsive="screen">
        <n-gi v-for="theme in themes" :key="theme.id">
          <n-card class="theme-card" :segmented="{ content: true, footer: true }">
            <template #header>
              <n-space align="center" :size="10">
                <span class="theme-name">{{ displayName(theme) }}</span>
                <n-tag v-if="theme.version" size="small" :bordered="false">v{{ theme.version }}</n-tag>
              </n-space>
            </template>
            <template #header-extra>
              <n-tag :type="theme.active ? 'success' : 'default'" size="small">{{ theme.active ? $t('themes.active') : $t('themes.inactive') }}</n-tag>
            </template>

            <p class="theme-description">{{ theme.description || $t('themes.no_description') }}</p>
            <div class="theme-meta">{{ theme.author || $t('themes.unknown_author') }}</div>

            <template #footer>
              <n-space justify="end">
                <n-button @click="openDetails(theme)">{{ $t('themes.details') }}</n-button>
                <n-button v-if="theme.active" type="primary" @click="router.push('/theme-settings')">{{ $t('themes.configure') }}</n-button>
                <n-button v-else type="primary" :loading="activating === theme.id" @click="activate(theme)">{{ $t('themes.activate') }}</n-button>
              </n-space>
            </template>
          </n-card>
        </n-gi>
      </n-grid>
      <n-empty v-else-if="!loading" :description="$t('themes.empty')" />
    </n-spin>

    <n-modal v-model:show="detailsVisible" preset="card" :title="selectedTheme ? displayName(selectedTheme) : ''" style="width: min(560px, calc(100vw - 32px))">
      <n-descriptions v-if="selectedTheme" :column="1" label-placement="left" bordered>
        <n-descriptions-item :label="$t('themes.identifier')">{{ selectedTheme.id }}</n-descriptions-item>
        <n-descriptions-item :label="$t('themes.version')">{{ selectedTheme.version || '-' }}</n-descriptions-item>
        <n-descriptions-item :label="$t('themes.author')">{{ selectedTheme.author || '-' }}</n-descriptions-item>
        <n-descriptions-item :label="$t('themes.description')">{{ selectedTheme.description || '-' }}</n-descriptions-item>
        <n-descriptions-item v-if="selectedTheme.homepage" :label="$t('themes.homepage')"><a :href="selectedTheme.homepage" target="_blank" rel="noreferrer">{{ selectedTheme.homepage }}</a></n-descriptions-item>
      </n-descriptions>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { settingsApi, type ThemeDetails } from '@/api/settings'

const { t } = useI18n()
const message = useMessage()
const router = useRouter()
const themes = ref<ThemeDetails[]>([])
const loading = ref(false)
const activating = ref<string | null>(null)
const detailsVisible = ref(false)
const selectedTheme = ref<ThemeDetails | null>(null)

function displayName(theme: ThemeDetails) {
  return theme.name || theme.id
}

function openDetails(theme: ThemeDetails) {
  selectedTheme.value = theme
  detailsVisible.value = true
}

async function fetchThemes() {
  loading.value = true
  try {
    const { data } = await settingsApi.getThemes()
    themes.value = data.data || []
  } catch {
    message.error(t('themes.fetch_failed'))
  } finally {
    loading.value = false
  }
}

async function activate(theme: ThemeDetails) {
  activating.value = theme.id
  try {
    await settingsApi.activateTheme(theme.id)
    await fetchThemes()
    message.success(t('themes.activate_success'))
  } catch {
    message.error(t('themes.activate_failed'))
  } finally {
    activating.value = null
  }
}

onMounted(fetchThemes)
</script>

<style scoped>
.theme-card {
  max-width: 920px;
}

.theme-name {
  font-size: 17px;
  font-weight: 600;
}

.theme-description {
  min-height: 24px;
  margin: 0 0 12px;
  line-height: 1.5;
}

.theme-meta {
  color: var(--n-text-color-3);
  font-size: 13px;
}
</style>

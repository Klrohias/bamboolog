<template>
  <n-space vertical size="large">
    <n-page-header :title="$t('theme_config.title')" :subtitle="themeName" />

    <n-spin :show="loading">
      <n-card v-if="configuration && configuration.schema.length" :title="$t('theme_config.form_title')">
        <n-form label-placement="top">
          <n-form-item v-for="field in configuration.schema" :key="field.key" :label="field.label" :show-feedback="false">
            <n-input
              v-if="field.type === 'string'"
              :value="stringValue(field.key)"
              :placeholder="field.description || undefined"
              @update:value="updateString(field.key, $event)"
            />
            <n-switch
              v-else-if="field.type === 'boolean'"
              :value="booleanValue(field.key)"
              @update:value="updateBoolean(field.key, $event)"
            />
            <n-input-number
              v-else-if="field.type === 'integer' || field.type === 'number'"
              :value="numberValue(field.key)"
              :min="field.min ?? undefined"
              :max="field.max ?? undefined"
              :precision="field.type === 'integer' ? 0 : undefined"
              style="width: 100%"
              @update:value="updateNumber(field.key, $event)"
            />
            <n-select
              v-else-if="field.type === 'select'"
              :value="stringValue(field.key)"
              :options="field.options.map(option => ({ label: option.label, value: option.value }))"
              @update:value="updateSelect(field.key, $event)"
            />
            <template #feedback>{{ field.description }}</template>
          </n-form-item>
          <n-button type="primary" :loading="saving" @click="save">{{ $t('common.save') }}</n-button>
        </n-form>
      </n-card>
      <n-empty v-else-if="configuration && !loading" :description="$t('theme_config.no_fields')" />
    </n-spin>
  </n-space>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { settingsApi, type ThemeConfigValue, type ThemeConfiguration } from '@/api/settings'

const { t } = useI18n()
const message = useMessage()
const configuration = ref<ThemeConfiguration | null>(null)
const values = ref<Record<string, ThemeConfigValue>>({})
const loading = ref(false)
const saving = ref(false)

const themeName = computed(() => {
  if (!configuration.value) return ''
  return configuration.value.theme.name || configuration.value.theme.id
})

function setValue(key: string, value: ThemeConfigValue) {
  values.value[key] = value
}

function updateString(key: string, value: string) {
  setValue(key, value)
}

function updateBoolean(key: string, value: boolean) {
  setValue(key, value)
}

function updateNumber(key: string, value: number | null) {
  if (value !== null) setValue(key, value)
}

function updateSelect(key: string, value: string | number | null) {
  if (typeof value === 'string') setValue(key, value)
}

function stringValue(key: string) {
  const value = values.value[key]
  return typeof value === 'string' ? value : ''
}

function booleanValue(key: string) {
  return values.value[key] === true
}

function numberValue(key: string) {
  const value = values.value[key]
  return typeof value === 'number' ? value : null
}

async function loadConfiguration() {
  loading.value = true
  try {
    const { data } = await settingsApi.getActiveThemeConfig()
    configuration.value = data.data
    values.value = { ...(data.data?.values || {}) }
  } catch {
    message.error(t('theme_config.fetch_failed'))
  } finally {
    loading.value = false
  }
}

async function save() {
  saving.value = true
  try {
    const { data } = await settingsApi.updateActiveThemeConfig(values.value)
    configuration.value = data.data
    values.value = { ...(data.data?.values || {}) }
    message.success(t('theme_config.save_success'))
  } catch {
    message.error(t('theme_config.save_failed'))
  } finally {
    saving.value = false
  }
}

onMounted(loadConfiguration)
</script>

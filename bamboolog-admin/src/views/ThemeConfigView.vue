<template>
  <n-space vertical size="large">
    <n-page-header :title="$t('theme_config.title')" :subtitle="themeName" />

    <n-spin :show="loading">
      <n-card v-if="configuration && configuration.schema.length" :title="$t('theme_config.form_title')">
        <div class="theme-config-form">
          <div v-for="field in configuration.schema" :key="field.key" class="theme-config-field">
            <span class="theme-config-field-label">{{ field.label }}</span>
            <div class="theme-config-field-control">
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
                @update:value="updateNumber(field.key, $event)"
              />
              <n-select
                v-else-if="field.type === 'select'"
                :value="stringValue(field.key)"
                :options="field.options.map(option => ({ label: option.label, value: option.value }))"
                @update:value="updateSelect(field.key, $event)"
              />
              <n-button v-else-if="field.type === 'json'" type="primary" text @click="openJsonEditor(field)">
                {{ $t('theme_config.edit_json') }}
              </n-button>
            </div>
          </div>
          <div class="theme-config-actions">
            <n-button type="primary" :loading="saving" @click="save">{{ $t('common.save') }}</n-button>
          </div>
        </div>
      </n-card>
      <n-empty v-else-if="configuration && !loading" :description="$t('theme_config.no_fields')" />
    </n-spin>

    <n-modal v-model:show="jsonEditorVisible" preset="card" :title="jsonField?.label" style="width: min(900px, calc(100vw - 32px))">
      <CodeEditor
        v-if="jsonEditorVisible"
        v-model="jsonDraft"
        language="json"
        height="min(60vh, 560px)"
        @update:model-value="jsonValidationError = ''"
      />
      <n-alert v-if="jsonValidationError" type="error" :show-icon="false" class="json-editor-error">
        {{ jsonValidationError }}
      </n-alert>
      <template #footer>
        <n-space justify="end">
          <n-button @click="jsonEditorVisible = false">{{ $t('common.cancel') }}</n-button>
          <n-button type="primary" @click="applyJsonValue">{{ $t('theme_config.apply_json') }}</n-button>
        </n-space>
      </template>
    </n-modal>
  </n-space>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { useI18n } from 'vue-i18n'
import { settingsApi, type ThemeConfigField, type ThemeConfigValue, type ThemeConfiguration } from '@/api/settings'

const CodeEditor = defineAsyncComponent(() => import('@/components/CodeEditor.vue'))

const { t } = useI18n()
const message = useMessage()
const configuration = ref<ThemeConfiguration | null>(null)
const values = ref<Record<string, ThemeConfigValue>>({})
const loading = ref(false)
const saving = ref(false)
const jsonEditorVisible = ref(false)
const jsonField = ref<ThemeConfigField | null>(null)
const jsonDraft = ref('null')
const jsonValidationError = ref('')

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

function openJsonEditor(field: ThemeConfigField) {
  jsonField.value = field
  jsonDraft.value = JSON.stringify(values.value[field.key] ?? null, null, 2)
  jsonValidationError.value = ''
  jsonEditorVisible.value = true
}

function applyJsonValue() {
  if (!jsonField.value) return
  try {
    setValue(jsonField.value.key, JSON.parse(jsonDraft.value) as ThemeConfigValue)
    jsonEditorVisible.value = false
  } catch {
    jsonValidationError.value = t('theme_config.invalid_json')
  }
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

<style scoped>
.theme-config-form {
  display: flex;
  flex-direction: column;
  gap: 24px;
}

.theme-config-field {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.theme-config-field-label {
  color: var(--n-text-color);
  font-size: 14px;
  font-weight: 500;
  line-height: 20px;
}

.theme-config-field-control :deep(.n-input),
.theme-config-field-control :deep(.n-input-number),
.theme-config-field-control :deep(.n-base-selection) {
  width: 100%;
}

.theme-config-actions {
  margin-top: 8px;
}

.json-editor-error {
  margin-top: 12px;
}
</style>

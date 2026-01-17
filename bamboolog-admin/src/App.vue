<template>
  <n-config-provider
    :theme="theme"
    :locale="naiveLocale"
    :date-locale="naiveDateLocale"
  >
    <n-global-style />
    <n-message-provider>
      <n-dialog-provider>
        <router-view />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { darkTheme, zhCN, dateZhCN, enUS, dateEnUS } from 'naive-ui'
import { useSettingsStore } from './stores/settings'

const settingsStore = useSettingsStore()

const theme = computed(() => settingsStore.theme === 'dark' ? darkTheme : null)
const naiveLocale = computed(() => settingsStore.locale === 'zh-CN' ? zhCN : enUS)
const naiveDateLocale = computed(() => settingsStore.locale === 'zh-CN' ? dateZhCN : dateEnUS)
</script>

<style>
body {
  margin: 0;
  font-family: Inter, system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial;
}
</style>


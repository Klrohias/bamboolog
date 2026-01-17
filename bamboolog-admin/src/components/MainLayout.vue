<template>
  <n-layout has-sider position="absolute" style="height: 100vh">
    <n-layout-sider
      bordered
      collapse-mode="width"
      :collapsed-width="64"
      :width="240"
      :collapsed="settingsStore.collapsed"
      show-trigger
      @collapse="settingsStore.collapsed = true"
      @expand="settingsStore.collapsed = false"
    >
      <div class="logo">
        <span v-if="!settingsStore.collapsed">Bamboolog Admin</span>
        <span v-else>B</span>
      </div>
      <n-menu
        :collapsed="settingsStore.collapsed"
        :collapsed-width="64"
        :collapsed-icon-size="22"
        :options="menuOptions"
        v-model:value="activeKey"
      />
    </n-layout-sider>
    <n-layout>
      <n-layout-header bordered style="padding: 0 24px; height: 64px; display: flex; align-items: center; justify-content: space-between">
        <n-breadcrumb>
          <n-breadcrumb-item>{{ $t('common.admin') }}</n-breadcrumb-item>
          <n-breadcrumb-item>{{ currentRouteLabel }}</n-breadcrumb-item>
        </n-breadcrumb>
        <n-space align="center">
          <n-button quaternary circle @click="settingsStore.toggleTheme">
            <template #icon>
              <n-icon v-if="settingsStore.theme === 'dark'"><sunny-outline /></n-icon>
              <n-icon v-else><moon-outline /></n-icon>
            </template>
          </n-button>
          <n-dropdown :options="languageOptions" @select="handleLanguageSelect">
            <n-button quaternary circle>
              <template #icon>
                <n-icon><language-outline /></n-icon>
              </template>
            </n-button>
          </n-dropdown>
          <n-button @click="handleLogout" ghost>{{ $t('common.logout') }}</n-button>
        </n-space>
      </n-layout-header>
      <n-layout-content content-style="padding: 24px;">
        <router-view />
      </n-layout-content>
    </n-layout>
  </n-layout>
</template>

<script setup lang="ts">
import { h, ref, computed, watch } from 'vue'
import { NIcon, type MenuOption } from 'naive-ui'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  BookOutline,
  SettingsOutline,
  MoonOutline,
  SunnyOutline,
  LanguageOutline
} from '@vicons/ionicons5'
import { setAuthToken } from '../api'
import { useSettingsStore } from '../stores/settings'

const { t, locale } = useI18n()
const settingsStore = useSettingsStore()
const route = useRoute()
const router = useRouter()
const activeKey = ref<string | null>(null)

function renderIcon(icon: any) {
  return () => h(NIcon, null, { default: () => h(icon) })
}

const menuOptions = computed<MenuOption[]>(() => [
  {
    label: () => h(RouterLink, { to: '/posts' }, { default: () => t('common.posts') }),
    key: 'posts',
    icon: renderIcon(BookOutline)
  },
  {
    label: () => h(RouterLink, { to: '/settings' }, { default: () => t('common.settings') }),
    key: 'settings',
    icon: renderIcon(SettingsOutline)
  }
])

const languageOptions = [
  { label: '简体中文', key: 'zh-CN' },
  { label: 'English', key: 'en-US' }
]

const currentRouteLabel = computed(() => {
  if (activeKey.value === 'posts') return t('common.posts')
  if (activeKey.value === 'settings') return t('common.settings')
  return t('common.dashboard')
})

watch(
  () => route.path,
  (path) => {
    if (path.startsWith('/posts')) activeKey.value = 'posts'
    else if (path.startsWith('/settings')) activeKey.value = 'settings'
    else activeKey.value = null
  },
  { immediate: true }
)

function handleLanguageSelect(key: 'zh-CN' | 'en-US') {
  settingsStore.locale = key
  locale.value = key
}

function handleLogout() {
  setAuthToken(null)
  router.push('/login')
}
</script>

<style scoped>
.logo {
  height: 64px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  font-weight: bold;
  overflow: hidden;
  white-space: nowrap;
}
</style>


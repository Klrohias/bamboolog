<template>
  <n-layout has-sider position="absolute" class="app-layout">
    <n-layout-sider class="desktop-sider" bordered collapse-mode="transform" :collapsed-width="0" :width="240"
      :collapsed="settingsStore.collapsed" show-trigger @collapse="settingsStore.collapsed = true"
      collapsed-trigger-style="right: -20px;" @expand="settingsStore.collapsed = false">
      <div class="logo">
        <span v-if="!settingsStore.collapsed">Bamboolog Admin</span>
        <span v-else>B</span>
      </div>
      <n-menu :collapsed="settingsStore.collapsed" :collapsed-width="0" :collapsed-icon-size="22" :options="menuOptions"
        v-model:value="activeKey" />
    </n-layout-sider>
    <n-layout>
      <n-layout-header class="app-header" bordered>
        <div class="header-leading">
          <n-popover v-model:show="mobileMenuVisible" trigger="click" placement="bottom-start" :show-arrow="false" :width="240">
            <template #trigger>
              <n-tooltip trigger="hover">
                <template #trigger>
                  <n-button class="mobile-menu-trigger" quaternary circle :aria-label="$t('common.menu')">
                    <template #icon><n-icon><menu-outline /></n-icon></template>
                  </n-button>
                </template>
                {{ $t('common.menu') }}
              </n-tooltip>
            </template>
            <n-menu :value="activeKey" :options="menuOptions" @update:value="handleMobileMenuSelect" />
          </n-popover>
          <n-breadcrumb>
            <n-breadcrumb-item>{{ $t('common.admin') }}</n-breadcrumb-item>
            <n-breadcrumb-item>{{ currentRouteLabel }}</n-breadcrumb-item>
          </n-breadcrumb>
        </div>
        <div class="header-actions header-actions--desktop">
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
            <n-dropdown :options="userOptions" @select="handleUserSelect">
              <n-button quaternary>
                <template #icon>
                  <n-icon><person-outline /></n-icon>
                </template>
                <span class="user-name">{{ userStore.user?.nickname || userStore.user?.username || 'User' }}</span>
              </n-button>
            </n-dropdown>
          </n-space>
        </div>
        <div class="header-actions header-actions--mobile">
          <n-popover v-model:show="mobileHeaderActionsVisible" trigger="click" placement="bottom-end" :show-arrow="false" :width="208">
            <template #trigger>
              <n-tooltip trigger="hover">
                <template #trigger>
                  <n-button quaternary circle :aria-label="$t('common.menu')">
                    <template #icon><n-icon><ellipsis-horizontal-outline /></n-icon></template>
                  </n-button>
                </template>
                {{ $t('common.menu') }}
              </n-tooltip>
            </template>
            <n-space vertical :size="4">
              <n-button block quaternary @click="handleMobileThemeToggle">
                <template #icon>
                  <n-icon v-if="settingsStore.theme === 'dark'"><sunny-outline /></n-icon>
                  <n-icon v-else><moon-outline /></n-icon>
                </template>
                {{ $t('common.theme') }}
              </n-button>
              <n-dropdown :options="languageOptions" placement="left-start" @select="handleMobileLanguageSelect">
                <n-button block quaternary>
                  <template #icon><n-icon><language-outline /></n-icon></template>
                  {{ $t('common.language') }}
                </n-button>
              </n-dropdown>
              <n-dropdown :options="userOptions" placement="left-start" @select="handleUserSelect">
                <n-button block quaternary>
                  <template #icon><n-icon><person-outline /></n-icon></template>
                  {{ userStore.user?.nickname || userStore.user?.username || 'User' }}
                </n-button>
              </n-dropdown>
            </n-space>
          </n-popover>
        </div>
      </n-layout-header>
      <n-layout-content :class="['app-content', { 'app-content--immersive': route.meta.immersive === true }]">
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
  LanguageOutline,
  PersonOutline,
  LogOutOutline,
  ImageOutline,
  CloudOutline,
  ColorPaletteOutline,
  OptionsOutline,
  MenuOutline,
  EllipsisHorizontalOutline
} from '@vicons/ionicons5'
import { clearCookieAuth } from '@/api'
import { userApi } from '@/api/user'
import { useSettingsStore } from '@/stores/settings'
import { useUserStore } from '@/stores/user'

const { t, locale } = useI18n()
const settingsStore = useSettingsStore()
const userStore = useUserStore()
const route = useRoute()
const router = useRouter()
const activeKey = ref<string | null>(null)
const mobileMenuVisible = ref(false)
const mobileHeaderActionsVisible = ref(false)

function renderIcon(icon: any) {
  return () => h(NIcon, null, { default: () => h(icon) })
}

const menuOptions = computed<MenuOption[]>(() => [
  {
    type: 'group',
    label: () => t('common.content'),
    key: 'content',
    children: [
      {
        label: () => h(RouterLink, { to: '/posts' }, { default: () => t('common.posts') }),
        key: 'posts',
        icon: renderIcon(BookOutline)
      },
      {
        label: () => h(RouterLink, { to: '/attachments' }, { default: () => t('common.attachments') }),
        key: 'attachments',
        icon: renderIcon(ImageOutline)
      }
    ]
  },
  {
    type: 'group',
    label: () => t('common.theme'),
    key: 'theme',
    children: [
      {
        label: () => h(RouterLink, { to: '/themes' }, { default: () => t('common.themes') }),
        key: 'themes',
        icon: renderIcon(ColorPaletteOutline)
      },
      {
        label: () => h(RouterLink, { to: '/theme-settings' }, { default: () => t('common.theme_config') }),
        key: 'theme-settings',
        icon: renderIcon(OptionsOutline)
      }
    ]
  },
  {
    type: 'group',
    label: () => t('common.system'),
    key: 'system',
    children: [
      {
        label: () => h(RouterLink, { to: '/settings' }, { default: () => t('common.site_settings') }),
        key: 'settings',
        icon: renderIcon(SettingsOutline)
      },
      {
        label: () => h(RouterLink, { to: '/storage-engines' }, { default: () => t('common.storage_engine') }),
        key: 'storage-engines',
        icon: renderIcon(CloudOutline)
      }
    ]
  }
])

const languageOptions = [
  { label: '简体中文', key: 'zh-CN' },
  { label: 'English', key: 'en-US' }
]

const userOptions = computed(() => [
  {
    label: t('common.profile'),
    key: 'profile',
    icon: renderIcon(PersonOutline)
  },
  {
    label: t('common.logout'),
    key: 'logout',
    icon: renderIcon(LogOutOutline)
  }
])

const currentRouteLabel = computed(() => {
  if (activeKey.value === 'posts') return t('common.posts')
  if (activeKey.value === 'settings') return t('common.site_settings')
  if (activeKey.value === 'themes') return t('common.themes')
  if (activeKey.value === 'theme-settings') return t('common.theme_config')
  if (activeKey.value === 'attachments') return t('common.attachments')
  if (activeKey.value === 'storage-engines') return t('common.storage_engine')
  return t('common.dashboard')
})

watch(
  () => route.path,
  (path) => {
    if (path.startsWith('/posts')) activeKey.value = 'posts'
    else if (path.startsWith('/settings')) activeKey.value = 'settings'
    else if (path.startsWith('/themes')) activeKey.value = 'themes'
    else if (path.startsWith('/theme-settings')) activeKey.value = 'theme-settings'
    else if (path.startsWith('/attachments')) activeKey.value = 'attachments'
    else if (path.startsWith('/storage-engines')) activeKey.value = 'storage-engines'
    else activeKey.value = null
  },
  { immediate: true }
)

function handleLanguageSelect(key: 'zh-CN' | 'en-US') {
  settingsStore.locale = key
  locale.value = key
}

function handleMobileThemeToggle() {
  settingsStore.toggleTheme()
  mobileHeaderActionsVisible.value = false
}

function handleMobileLanguageSelect(key: 'zh-CN' | 'en-US') {
  handleLanguageSelect(key)
  mobileHeaderActionsVisible.value = false
}

function handleMobileMenuSelect(key: string | number) {
  activeKey.value = String(key)
  mobileMenuVisible.value = false
}

function handleUserSelect(key: string) {
  mobileHeaderActionsVisible.value = false
  if (key === 'profile') {
    router.push('/profile')
  } else if (key === 'logout') {
    handleLogout()
  }
}

async function handleLogout() {
  try {
    await userApi.logout()
  } finally {
    clearCookieAuth()
    userStore.logout()
    router.push('/login')
  }
}
</script>

<style scoped>
.app-layout {
  height: 100vh;
}

.app-header {
  height: 64px;
  padding: 0 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.header-leading {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
}

.mobile-menu-trigger {
  display: none;
}

.header-actions--mobile {
  display: none;
}

.app-content {
  padding-inline: 40px;
  padding-block: 30px;
}

.app-content.app-content--immersive {
  padding: 0;
}

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

@media (max-width: 767px) {
  .desktop-sider {
    display: none;
  }

  .app-header {
    padding-inline: 16px;
  }

  .mobile-menu-trigger {
    display: inline-flex;
  }

  .header-actions--desktop {
    display: none;
  }

  .header-actions--mobile {
    display: block;
  }

  .app-content {
    padding-inline: 16px;
  }

}
</style>

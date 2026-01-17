import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export const useSettingsStore = defineStore('settings', () => {
    const theme = ref<'light' | 'dark'>(localStorage.getItem('theme') as 'light' | 'dark' || 'light')
    const locale = ref<'zh-CN' | 'en-US'>(localStorage.getItem('locale') as 'zh-CN' | 'en-US' || 'zh-CN')
    const collapsed = ref(localStorage.getItem('sidebar_collapsed') === 'true')

    watch(theme, (newTheme) => {
        localStorage.setItem('theme', newTheme)
    })

    watch(locale, (newLocale) => {
        localStorage.setItem('locale', newLocale)
    })

    watch(collapsed, (newCollapsed) => {
        localStorage.setItem('sidebar_collapsed', String(newCollapsed))
    })

    const toggleTheme = () => {
        theme.value = theme.value === 'light' ? 'dark' : 'light'
    }

    return {
        theme,
        locale,
        collapsed,
        toggleTheme
    }
})

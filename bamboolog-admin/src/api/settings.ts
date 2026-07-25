import api, { type ApiResponse } from './index'

export interface SiteSettings {
    site_name: string
    base_url: string
    copyright: string
    description: string
    language: string
    favicon_url: string
    rss_enabled: boolean
    sitemap_enabled: boolean
    posts_per_page: number
}

export interface Settings {
    site: SiteSettings
}

export interface ThemeDetails {
    id: string
    active: boolean
    name: string | null
    version: string | null
    description: string | null
    homepage: string | null
    author: string | null
}

export type ThemeConfigValue = string | number | boolean | null | ThemeConfigValue[] | { [key: string]: ThemeConfigValue }

export interface ThemeConfigOption {
    label: string
    value: string
}

export interface ThemeConfigField {
    key: string
    label: string
    description: string | null
    type: 'string' | 'boolean' | 'integer' | 'number' | 'select' | 'json'
    required: boolean
    options: ThemeConfigOption[]
    min: number | null
    max: number | null
}

export interface ThemeConfiguration {
    theme: ThemeDetails
    schema: ThemeConfigField[]
    values: Record<string, ThemeConfigValue>
}

export const settingsApi = {
    get: () => {
        return api.get<ApiResponse<Settings>>('/settings/')
    },

    getThemes: () => {
        return api.get<ApiResponse<ThemeDetails[]>>('/settings/themes')
    },

    uploadTheme: (file: File) => {
        const formData = new FormData()
        formData.append('file', file)
        return api.post<ApiResponse<ThemeDetails>>('/settings/themes', formData, {
            headers: { 'Content-Type': 'multipart/form-data' }
        })
    },

    activateTheme: (theme: string) => {
        return api.post<ApiResponse<{ current: string }>>(`/settings/themes/${encodeURIComponent(theme)}/activate`)
    },

    getActiveThemeConfig: () => {
        return api.get<ApiResponse<ThemeConfiguration>>('/settings/themes/active/config')
    },

    updateActiveThemeConfig: (values: Record<string, ThemeConfigValue>) => {
        return api.post<ApiResponse<ThemeConfiguration>>('/settings/themes/active/config', { values })
    },

    update: (data: Settings) => {
        return api.post<ApiResponse<Settings>>('/settings/', data)
    }
}

import api, { type ApiResponse } from './index'

export interface Settings {
    // Add known settings keys here if needed, or keep generic
    [key: string]: any
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

export type ThemeConfigValue = string | number | boolean

export interface ThemeConfigOption {
    label: string
    value: string
}

export interface ThemeConfigField {
    key: string
    label: string
    description: string | null
    type: 'string' | 'boolean' | 'integer' | 'number' | 'select'
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

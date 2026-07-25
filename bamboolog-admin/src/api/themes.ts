import api, { type ApiResponse } from './index'

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

export const themesApi = {
    list: () => api.get<ApiResponse<ThemeDetails[]>>('/themes'),

    upload: (file: File) => {
        const formData = new FormData()
        formData.append('file', file)
        return api.post<ApiResponse<ThemeDetails>>('/themes', formData, {
            headers: { 'Content-Type': 'multipart/form-data' }
        })
    },

    activate: (theme: string) =>
        api.post<ApiResponse<{ current: string }>>(`/themes/${encodeURIComponent(theme)}/activate`),

    delete: (theme: string) => api.delete<ApiResponse<void>>(`/themes/${encodeURIComponent(theme)}`),

    getActiveConfig: () => api.get<ApiResponse<ThemeConfiguration>>('/themes/active/config'),

    updateActiveConfig: (values: Record<string, ThemeConfigValue>) =>
        api.post<ApiResponse<ThemeConfiguration>>('/themes/active/config', { values })
}

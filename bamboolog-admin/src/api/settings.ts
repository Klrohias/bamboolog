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

export const settingsApi = {
    get: () => {
        return api.get<ApiResponse<Settings>>('/settings/')
    },

    update: (data: Settings) => {
        return api.post<ApiResponse<Settings>>('/settings/', data)
    }
}

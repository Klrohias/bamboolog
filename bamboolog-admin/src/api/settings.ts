import api, { type ApiResponse } from './index'

export interface Settings {
    // Add known settings keys here if needed, or keep generic
    [key: string]: any
}

export const settingsApi = {
    get: () => {
        return api.get<ApiResponse<Settings>>('/settings/')
    },

    getThemes: () => {
        return api.get<ApiResponse<string[]>>('/settings/themes')
    },

    update: (data: Settings) => {
        return api.post<ApiResponse<Settings>>('/settings/', data)
    }
}

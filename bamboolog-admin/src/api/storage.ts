import api, { type ApiResponse } from './index'

export interface StorageEngine {
    id: number
    name: string
    comments: string
    kind: 'local' | 's3'
    config_json?: string
    is_default: boolean
    enabled: boolean
}

export const storageApi = {
    list: () => {
        return api.get<ApiResponse<StorageEngine[]>>('/storage_engines')
    },

    create: (data: Omit<StorageEngine, 'id'>) => {
        return api.post<ApiResponse<StorageEngine>>('/storage_engines', data)
    },

    update: (id: number, data: Partial<Omit<StorageEngine, 'id'>>) => {
        return api.put<ApiResponse<StorageEngine>>(`/storage_engines/${id}`, data)
    },

    delete: (id: number) => {
        return api.delete<ApiResponse<void>>(`/storage_engines/${id}`)
    }
}

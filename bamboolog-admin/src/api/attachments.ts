import api, { type ApiResponse } from './index'

export interface Attachment {
    id: number
    mime: string
    hash: string
    storage_engine_id: number
    path: string
    created_at: string
}

export interface AttachmentListResponse {
    items: Attachment[]
    total: number
    page: number
    size: number
    total_pages: number
}

export const attachmentApi = {
    list: (page: number = 1, size: number = 20, mime?: string, storage_engine_id?: number, sort?: string, order?: string) => {
        return api.get<ApiResponse<AttachmentListResponse>>('/attachments/', {
            params: { page, size, mime, storage_engine_id, sort, order }
        })
    },

    upload: (file: File, storage_engine_id?: number) => {
        const formData = new FormData()
        formData.append('file', file)
        if (storage_engine_id) {
            formData.append('storage_engine_id', storage_engine_id.toString())
        }
        return api.post<ApiResponse<Attachment>>('/attachments/', formData, {
            headers: {
                'Content-Type': 'multipart/form-data'
            }
        })
    },

    delete: (id: number) => {
        return api.delete<ApiResponse<void>>(`/attachments/${id}`)
    }
}

import api, { type ApiResponse } from './index'

export interface Post {
    id: number
    title: string
    name: string
    content: string
    created_at: string
    updated_at: string
}

export interface PostListResponse {
    posts: Post[]
    total: number
    page: number
    page_size: number
    total_pages: number
}

export interface PostListParams {
    page?: number
    page_size?: number
    sort_by?: string
    order?: 'asc' | 'desc'
    title?: string
    name?: string
}

export const postsApi = {
    list: (params: PostListParams) => {
        return api.get<ApiResponse<PostListResponse>>('/posts/', { params })
    },

    get: (id: number) => {
        return api.get<ApiResponse<Post>>(`/posts/${id}`)
    },

    create: (data: Partial<Post>) => {
        return api.put<ApiResponse<Post>>('/posts/', data)
    },

    update: (id: number, data: Partial<Post>) => {
        return api.post<ApiResponse<Post>>(`/posts/${id}`, data)
    },

    delete: (id: number) => {
        return api.delete<ApiResponse<void>>(`/posts/${id}`)
    }
}

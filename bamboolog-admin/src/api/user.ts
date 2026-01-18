import api, { type ApiResponse } from './index'

export interface User {
    id: number
    username: string
    nickname: string
    role: string
    // other fields
}

export interface LoginRequest {
    username: string
    password: string
}

export interface LoginResponse {
    token: string
    user: User
}

export interface UpdateProfileRequest {
    nickname?: string
    old_password?: string
    new_password?: string
}

export const userApi = {
    login: (data: LoginRequest) => {
        return api.post<ApiResponse<LoginResponse>>('/user/auth', data)
    },

    getMe: () => {
        return api.get<ApiResponse<User>>('/user/me')
    },

    updateProfile: (data: UpdateProfileRequest) => {
        return api.post<ApiResponse<User>>('/user/me', data)
    }
}

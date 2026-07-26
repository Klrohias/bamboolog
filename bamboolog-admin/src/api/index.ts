import axios from 'axios'
import router from '@/router'

const baseURL = import.meta.env.VITE_API_BASE || '/api'

const api = axios.create({
    baseURL,
    withCredentials: true,
    headers: {
        'Content-Type': 'application/json',
    },
})

export interface ApiResponse<T> {
    code: number // or status depending on backend
    message: string
    data: T
}

export function useCookieAuth() {
    api.defaults.headers.common['Authorization'] = 'cookie'
}

export function clearCookieAuth() {
    delete api.defaults.headers.common['Authorization']
}

api.interceptors.response.use(
    (response) => response,
    async (error) => {
        if (error.response && error.response.status === 401) {
            try {
                const { useUserStore } = await import('@/stores/user')
                const userStore = useUserStore()
                userStore.logout()
            } catch (e) {
                // Ignore if store cannot be loaded
            }
            clearCookieAuth()
            router.push('/login')
        }
        return Promise.reject(error)
    }
)

useCookieAuth()

export default api

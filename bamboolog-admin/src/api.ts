import axios from 'axios'
import router from './router'

const baseURL = import.meta.env.VITE_API_BASE || '/api'

const api = axios.create({
    baseURL,
    headers: {
        'Content-Type': 'application/json',
    },
})

export function setAuthToken(token: string | null) {
    if (token) {
        api.defaults.headers.common['Authorization'] = `Bearer ${token}`
        localStorage.setItem('token', token)
    } else {
        delete api.defaults.headers.common['Authorization']
        localStorage.removeItem('token')
    }
}

api.interceptors.response.use(
    (response) => response,
    (error) => {
        if (error.response && error.response.status === 401) {
            setAuthToken(null)
            router.push('/login')
        }
        return Promise.reject(error)
    }
)

const saved = localStorage.getItem('token')
if (saved) setAuthToken(saved)

export default api

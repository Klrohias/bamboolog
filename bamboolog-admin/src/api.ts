import axios from 'axios'

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

const saved = localStorage.getItem('token')
if (saved) setAuthToken(saved)

export default api

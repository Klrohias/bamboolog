import { defineStore } from 'pinia'
import { ref } from 'vue'
import api from '@/api'

export const useUserStore = defineStore('user', () => {
    const user = ref<any>(null)
    const initialized = ref(false)

    async function fetchMe() {
        try {
            const { data } = await api.get('/user/me')
            user.value = data.data
            return true
        } catch (e) {
            user.value = null
            return false
        } finally {
            initialized.value = true
        }
    }

    async function updateProfile(payload: any) {
        const { data } = await api.post('/user/me', payload)
        user.value = data.data
        return data.data
    }

    function logout() {
        user.value = null
        // setAuthToken(null) will be called by whatever calls logout
        // or logout can call setAuthToken(null) if imported
    }

    return {
        user,
        initialized,
        fetchMe,
        logout,
        updateProfile
    }
})

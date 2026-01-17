import { createRouter, createWebHistory } from 'vue-router'
import Login from './views/Login.vue'
import MainLayout from './components/MainLayout.vue'
import PostsView from './views/PostsView.vue'
import PostEditor from './views/PostEditor.vue'
import SettingsView from './views/SettingsView.vue'

const routes = [
    { path: '/login', component: Login, name: 'Login' },
    {
        path: '/',
        component: MainLayout,
        children: [
            { path: '', redirect: '/posts' },
            { path: 'posts', component: PostsView, name: 'Posts' },
            { path: 'posts/new', component: PostEditor, name: 'New Post' },
            { path: 'posts/edit/:id', component: PostEditor, name: 'Edit Post' },
            { path: 'settings', component: SettingsView, name: 'Settings' },
        ]
    }
]

const router = createRouter({
    history: createWebHistory(),
    routes,
})

export default router

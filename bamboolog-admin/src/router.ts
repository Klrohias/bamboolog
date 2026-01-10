import Login from './views/Login.vue'
import PostsAdmin from './views/PostsAdmin.vue'

export default [
    { path: '/', redirect: '/posts' },
    { path: '/login', component: Login },
    { path: '/posts', component: PostsAdmin },
]

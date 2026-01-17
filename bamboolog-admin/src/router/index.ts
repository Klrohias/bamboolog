import { createRouter, createWebHistory } from 'vue-router'
import Login from '@/views/Login.vue'
import MainLayout from '@/components/MainLayout.vue'
import PostsView from '@/views/PostsView.vue'
import PostEditor from '@/views/PostEditor.vue'
import SettingsView from '@/views/SettingsView.vue'
import { useUserStore } from '@/stores/user'

const routes = [
  { path: '/login', component: Login, name: 'Login', meta: { public: true } },
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
  history: createWebHistory(import.meta.env.BASE_URL),
  routes,
})

router.beforeEach(async (to, _from, next) => {
  const userStore = useUserStore()

  if (!userStore.initialized) {
    await userStore.fetchMe()
  }

  if (to.name === 'Login' && userStore.user) {
    next('/posts')
  } else if (!to.meta.public && !userStore.user) {
    next('/login')
  } else {
    next()
  }
})

export default router

<template>
  <div class="user-profile">
    <n-page-header>
      <template #title>
        {{ $t('common.profile') }}
      </template>
    </n-page-header>

    <div class="content">
      <n-grid :x-gap="24" :y-gap="24" cols="1 s:1 m:2" responsive="screen">
        <n-grid-item>
          <n-card :title="$t('profile.basicInfo')">
            <n-form
              ref="profileFormRef"
              :model="profileModel"
              :rules="profileRules"
              label-placement="top"
            >
              <n-form-item :label="$t('profile.username')">
                <n-input :value="userStore.user?.username" disabled />
              </n-form-item>
              <n-form-item :label="$t('profile.nickname')" path="nickname">
                <n-input v-model:value="profileModel.nickname" :placeholder="$t('profile.nicknamePlaceholder')" />
              </n-form-item>
              <n-button type="primary" :loading="profileLoading" @click="handleUpdateProfile">
                {{ $t('common.save') }}
              </n-button>
            </n-form>
          </n-card>
        </n-grid-item>

        <n-grid-item>
          <n-card :title="$t('profile.security')">
            <n-form
              ref="passwordFormRef"
              :model="passwordModel"
              :rules="passwordRules"
              label-placement="top"
            >
              <n-form-item :label="$t('profile.oldPassword')" path="oldPassword">
                <n-input
                  v-model:value="passwordModel.oldPassword"
                  type="password"
                  show-password-on="click"
                  :placeholder="$t('profile.oldPasswordPlaceholder')"
                />
              </n-form-item>
              <n-form-item :label="$t('profile.newPassword')" path="newPassword">
                <n-input
                  v-model:value="passwordModel.newPassword"
                  type="password"
                  show-password-on="click"
                  :placeholder="$t('profile.newPasswordPlaceholder')"
                />
              </n-form-item>
              <n-form-item :label="$t('profile.confirmPassword')" path="confirmPassword">
                <n-input
                  v-model:value="passwordModel.confirmPassword"
                  type="password"
                  show-password-on="click"
                  :placeholder="$t('profile.confirmPasswordPlaceholder')"
                />
              </n-form-item>
              <n-button type="primary" :loading="passwordLoading" @click="handleUpdatePassword">
                {{ $t('common.changePassword') }}
              </n-button>
            </n-form>
          </n-card>
        </n-grid-item>
      </n-grid>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useUserStore } from '@/stores/user'
import { useMessage, type FormInst, type FormRules } from 'naive-ui'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()
const userStore = useUserStore()
const message = useMessage()

const profileFormRef = ref<FormInst | null>(null)
const passwordFormRef = ref<FormInst | null>(null)
const profileLoading = ref(false)
const passwordLoading = ref(false)

const profileModel = reactive({
  nickname: ''
})

const passwordModel = reactive({
  oldPassword: '',
  newPassword: '',
  confirmPassword: ''
})

const profileRules: FormRules = {
  nickname: [
    { required: true, message: () => t('profile.nicknameRequired'), trigger: 'blur' }
  ]
}

const passwordRules: FormRules = {
  oldPassword: [
    { required: true, message: () => t('profile.oldPasswordRequired'), trigger: 'blur' }
  ],
  newPassword: [
    { required: true, message: () => t('profile.newPasswordRequired'), trigger: 'blur' },
    { min: 6, message: () => t('profile.passwordMinLength'), trigger: 'blur' }
  ],
  confirmPassword: [
    { required: true, message: () => t('profile.confirmPasswordRequired'), trigger: 'blur' },
    {
      validator: (_rule: any, value: string) => {
        return value === passwordModel.newPassword
      },
      message: () => t('profile.passwordMismatch'),
      trigger: 'blur'
    }
  ]
}

onMounted(() => {
  if (userStore.user) {
    profileModel.nickname = userStore.user.nickname
  }
})

async function handleUpdateProfile() {
  try {
    await profileFormRef.value?.validate()
    profileLoading.value = true
    await userStore.updateProfile({ nickname: profileModel.nickname })
    message.success(t('common.success'))
  } catch (e: any) {
    if (e.message) return // Validation failed
    message.error(t('common.error'))
  } finally {
    profileLoading.value = false
  }
}

async function handleUpdatePassword() {
  try {
    await passwordFormRef.value?.validate()
    passwordLoading.value = true
    await userStore.updateProfile({
      old_password: passwordModel.oldPassword,
      new_password: passwordModel.newPassword
    })
    message.success(t('common.success'))
    passwordModel.oldPassword = ''
    passwordModel.newPassword = ''
    passwordModel.confirmPassword = ''
  } catch (e: any) {
    if (e.response?.data?.message) {
        message.error(e.response.data.message)
    } else if (!e.message) {
         // Validation failed likely
    } else {
        message.error(t('common.error'))
    }
  } finally {
    passwordLoading.value = false
  }
}
</script>

<style scoped>
.content {
  margin-top: 24px;
}
</style>

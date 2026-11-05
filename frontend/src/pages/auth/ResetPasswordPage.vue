<template>
  <form class="auth-form" novalidate @submit.prevent="submit">
    <h1>Đặt mật khẩu mới</h1>
    <hui-alert v-if="!token" severity="error" description="Link không hợp lệ. Hãy mở lại link trong email." />
    <template v-else>
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input
        v-model="password"
        type="password"
        label="Mật khẩu mới"
        hint="Ít nhất 8 ký tự, có chữ và số"
        autocomplete="new-password"
        :error="fields.newPassword"
        required
      />
      <hui-input v-model="confirm" type="password" label="Nhập lại mật khẩu" autocomplete="new-password" :error="confirmError" required />
      <hui-button type="submit" block :loading="loading">Đặt mật khẩu</hui-button>
    </template>
    <div class="auth-links">
      <RouterLink to="/quen-mat-khau">Gửi lại link</RouterLink>
      <RouterLink to="/dang-nhap">Đăng nhập</RouterLink>
    </div>
  </form>
</template>

<script setup>
import { ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { authApi } from '../../api';
import { useAuthStore } from '../../stores/auth';
import { useSubmit } from '../../utils/form';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const token = typeof route.query.token === 'string' ? route.query.token : '';
const password = ref('');
const confirm = ref('');
const confirmError = ref('');
const { loading, error, fields, run } = useSubmit();

async function submit() {
  confirmError.value = password.value !== confirm.value ? 'Mật khẩu nhập lại không khớp.' : '';
  if (confirmError.value) return;
  const res = await run(() => authApi.resetPassword(token, password.value));
  if (res.ok) {
    // Mọi phiên cũ đã bị thu hồi ở server.
    auth.clear();
    router.replace({ name: 'login', query: { reset: '1' } });
  }
}
</script>

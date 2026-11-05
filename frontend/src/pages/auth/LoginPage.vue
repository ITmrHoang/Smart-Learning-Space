<template>
  <form class="auth-form" novalidate @submit.prevent="submit">
    <h1>Đăng nhập</h1>
    <hui-alert v-if="error" severity="error" :description="error" />
    <hui-alert v-if="route.query.reset === '1'" severity="success" description="Đã đặt mật khẩu mới. Hãy đăng nhập lại." />

    <hui-input
      v-model="form.login"
      label="Username hoặc email"
      autocomplete="username"
      :error="fields.login"
      required
    />
    <hui-input
      v-model="form.password"
      type="password"
      label="Mật khẩu"
      autocomplete="current-password"
      :error="fields.password"
      required
    />
    <hui-button type="submit" block :loading="loading">Đăng nhập</hui-button>

    <div class="auth-links">
      <RouterLink to="/quen-mat-khau">Quên mật khẩu?</RouterLink>
      <RouterLink to="/dang-ky">Chưa có tài khoản? Đăng ký</RouterLink>
    </div>
  </form>
</template>

<script setup>
import { reactive } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { useAuthStore } from '../../stores/auth';
import { useSubmit } from '../../utils/form';

const auth = useAuthStore();
const route = useRoute();
const router = useRouter();
const form = reactive({ login: '', password: '' });
const { loading, error, fields, run } = useSubmit();

async function submit() {
  const res = await run(() => auth.login(form.login.trim(), form.password));
  if (res.ok) {
    const redirect = typeof route.query.redirect === 'string' && route.query.redirect.startsWith('/') ? route.query.redirect : '/';
    router.replace(redirect);
  } else if (res.error?.code === 'EMAIL_NOT_VERIFIED') {
    const email = form.login.includes('@') ? form.login.trim() : undefined;
    router.push({ name: 'verify-email', query: email ? { email } : {} });
  }
}
</script>

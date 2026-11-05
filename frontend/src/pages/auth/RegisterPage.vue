<template>
  <form class="auth-form" novalidate @submit.prevent="submit">
    <h1>Tạo tài khoản học sinh</h1>
    <p class="muted small">Giáo viên cũng đăng ký tại đây, sau đó gửi hồ sơ giáo viên để được duyệt.</p>
    <hui-alert v-if="error" severity="error" :description="error" />

    <hui-input v-model="form.fullName" label="Họ và tên" autocomplete="name" :error="fields.fullName" required />
    <hui-input
      v-model="form.username"
      label="Username"
      hint="3–32 ký tự: chữ thường không dấu, số, dấu chấm, gạch dưới"
      autocomplete="username"
      :error="fields.username"
      required
    />
    <hui-input v-model="form.email" type="email" label="Email" autocomplete="email" :error="fields.email" required />
    <hui-input
      v-model="form.password"
      type="password"
      label="Mật khẩu"
      hint="Ít nhất 8 ký tự, có chữ và số"
      autocomplete="new-password"
      :error="fields.password"
      required
    />
    <hui-input
      v-model="form.confirm"
      type="password"
      label="Nhập lại mật khẩu"
      autocomplete="new-password"
      :error="confirmError"
      required
    />
    <hui-button type="submit" block :loading="loading">Đăng ký</hui-button>

    <div class="auth-links">
      <RouterLink to="/dang-nhap">Đã có tài khoản? Đăng nhập</RouterLink>
    </div>
  </form>
</template>

<script setup>
import { reactive, ref } from 'vue';
import { useRouter } from 'vue-router';

import { authApi } from '../../api';
import { useSubmit } from '../../utils/form';

const router = useRouter();
const form = reactive({ fullName: '', username: '', email: '', password: '', confirm: '' });
const confirmError = ref('');
const { loading, error, fields, run } = useSubmit();

async function submit() {
  confirmError.value = form.password !== form.confirm ? 'Mật khẩu nhập lại không khớp.' : '';
  if (confirmError.value) return;
  const res = await run(() =>
    authApi.register({
      fullName: form.fullName.trim(),
      username: form.username.trim().toLowerCase(),
      email: form.email.trim(),
      password: form.password,
    }),
  );
  if (res.ok) router.push({ name: 'verify-email', query: { email: res.data.email } });
}
</script>

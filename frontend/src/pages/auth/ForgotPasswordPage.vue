<template>
  <form class="auth-form" novalidate @submit.prevent="submit">
    <h1>Quên mật khẩu</h1>
    <template v-if="!sent">
      <p class="muted small">Nhập email đã đăng ký, chúng tôi sẽ gửi link đặt lại mật khẩu (hiệu lực 30 phút).</p>
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input v-model="email" type="email" label="Email" autocomplete="email" :error="fields.email" required />
      <hui-button type="submit" block :loading="loading">Gửi link đặt lại</hui-button>
    </template>
    <hui-alert
      v-else
      severity="success"
      title="Đã gửi yêu cầu"
      description="Nếu email này đã đăng ký, bạn sẽ nhận được link đặt lại mật khẩu trong vài phút. Hãy kiểm tra cả hộp thư rác."
    />
    <div class="auth-links">
      <RouterLink to="/dang-nhap">Quay lại đăng nhập</RouterLink>
    </div>
  </form>
</template>

<script setup>
import { ref } from 'vue';

import { authApi } from '../../api';
import { useSubmit } from '../../utils/form';

const email = ref('');
const sent = ref(false);
const { loading, error, fields, run } = useSubmit();

async function submit() {
  const res = await run(() => authApi.forgotPassword(email.value.trim()));
  if (res.ok) sent.value = true;
}
</script>

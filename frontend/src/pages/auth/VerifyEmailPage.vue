<template>
  <form class="auth-form" novalidate @submit.prevent="submit">
    <h1>Xác minh email</h1>
    <p class="muted small">
      Nhập mã 6 số vừa gửi tới email của bạn. Mã có hiệu lực 15 phút.
    </p>
    <hui-alert v-if="error" severity="error" :description="error" />
    <hui-alert v-if="notice" severity="info" :description="notice" />

    <hui-input v-model="form.email" type="email" label="Email" autocomplete="email" :error="fields.email" required />
    <hui-otp-input
      :value="form.code"
      label="Mã xác minh"
      :length="6"
      @input="form.code = $event.detail.value"
      @hui-complete="onComplete"
    />
    <p v-if="fields.code" class="field-error small">{{ fields.code }}</p>

    <hui-button type="submit" block :loading="loading" :disabled="form.code.length !== 6">Xác minh</hui-button>

    <div class="auth-links">
      <hui-button variant="link" size="sm" :disabled="cooldown > 0 || !form.email" @hui-click="resend">
        {{ cooldown > 0 ? `Gửi lại mã sau ${cooldown}s` : 'Gửi lại mã' }}
      </hui-button>
      <RouterLink to="/dang-nhap">Quay lại đăng nhập</RouterLink>
    </div>
  </form>
</template>

<script setup>
import { onBeforeUnmount, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { authApi } from '../../api';
import { useAuthStore } from '../../stores/auth';
import { useSubmit } from '../../utils/form';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const form = reactive({ email: typeof route.query.email === 'string' ? route.query.email : '', code: '' });
const notice = ref('');
const cooldown = ref(route.query.email ? 60 : 0);
const { loading, error, fields, run } = useSubmit();

const timer = setInterval(() => {
  if (cooldown.value > 0) cooldown.value -= 1;
}, 1000);
onBeforeUnmount(() => clearInterval(timer));

function onComplete(e) {
  form.code = e.detail.value;
  submit();
}

async function submit() {
  if (loading.value || form.code.length !== 6) return;
  notice.value = '';
  const res = await run(() => auth.verifyEmail(form.email.trim(), form.code));
  if (res.ok) router.replace({ name: 'home' });
}

async function resend() {
  notice.value = '';
  try {
    await authApi.resendVerification(form.email.trim());
    notice.value = 'Nếu email đang chờ xác minh, mã mới đã được gửi.';
    cooldown.value = 60;
  } catch (e) {
    error.value = e.message;
  }
}
</script>

<style scoped>
.field-error {
  margin: -6px 0 0;
  color: var(--hui-danger, #d4380d);
}
</style>

<template>
  <form class="auth-form" novalidate @submit.prevent="submit">
    <h1>Đăng nhập</h1>
    <hui-alert v-if="error" severity="error" :description="error" />
    <hui-alert v-else-if="ssoError" severity="error" :description="ssoError" />
    <hui-alert v-else-if="endedMessage" severity="warning" :description="endedMessage" />
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
    <SsoButtons :next="redirectTo" />

    <div class="auth-links">
      <RouterLink to="/quen-mat-khau">Quên mật khẩu?</RouterLink>
      <RouterLink to="/dang-ky">Chưa có tài khoản? Đăng ký</RouterLink>
    </div>
  </form>
</template>

<script setup>
import { computed, reactive } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import SsoButtons from '../../components/SsoButtons.vue';
import { useAuthStore } from '../../stores/auth';
import { useSubmit } from '../../utils/form';
import { safeNext, sessionEndedMessage, ssoErrorMessage } from '../../utils/sso';

const auth = useAuthStore();
const route = useRoute();
const router = useRouter();
const form = reactive({ login: '', password: '' });
const { loading, error, fields, run } = useSubmit();
const redirectTo = computed(() => safeNext(route.query.redirect));
const ssoError = computed(() => ssoErrorMessage(typeof route.query.ssoError === 'string' ? route.query.ssoError : ''));
const endedMessage = computed(() => sessionEndedMessage(auth.endedReason));

async function submit() {
  const res = await run(() => auth.login(form.login.trim(), form.password));
  if (res.ok) {
    router.replace(redirectTo.value || '/');
  } else if (res.error?.code === 'EMAIL_NOT_VERIFIED') {
    const email = form.login.includes('@') ? form.login.trim() : undefined;
    router.push({ name: 'verify-email', query: email ? { email } : {} });
  }
}
</script>

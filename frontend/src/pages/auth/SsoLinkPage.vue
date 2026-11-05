<template>
  <div class="auth-form">
    <h1>Liên kết tài khoản {{ providerLabel }}</h1>

    <template v-if="!info || ended">
      <hui-alert severity="error" :description="error || 'Yêu cầu liên kết không hợp lệ hoặc đã hết hạn.'" />
      <hui-button block @hui-click="router.replace({ name: 'login' })">Về trang đăng nhập</hui-button>
    </template>

    <form v-else class="auth-form" novalidate @submit.prevent="submit">
      <p class="muted small">
        Email <strong>{{ info.emailHint }}</strong> đã có tài khoản trên Smart Learning Space. Để đăng nhập bằng
        {{ providerLabel }}, hãy xác nhận bạn là chủ tài khoản này. Chỉ cần làm một lần.
      </p>
      <hui-alert v-if="error" severity="error" :description="error" />

      <hui-segmented v-if="info.canUsePassword" block label="Cách xác nhận" :options="methodOptions" :value="method" @change="setMethod($event.detail.value)" />

      <template v-if="method === 'password'">
        <hui-input
          v-model="password"
          type="password"
          label="Mật khẩu của tài khoản"
          autocomplete="current-password"
          :error="fields.password"
          required
        />
      </template>
      <template v-else>
        <hui-button variant="outline" block :loading="sending" :disabled="cooldown > 0" @hui-click="sendCode">
          {{ cooldown > 0 ? `Gửi lại mã sau ${cooldown}s` : codeSent ? 'Gửi lại mã' : `Gửi mã tới ${info.emailHint}` }}
        </hui-button>
        <hui-alert v-if="codeSent" severity="info" :description="`Đã gửi mã 6 số tới ${info.emailHint}. Mã có hiệu lực tới khi yêu cầu liên kết hết hạn.`" />
        <hui-otp-input
          v-if="codeSent"
          :value="code"
          label="Mã xác minh"
          :length="6"
          @input="code = $event.detail.value"
          @hui-complete="onComplete"
        />
        <p v-if="fields.code" class="field-error small">{{ fields.code }}</p>
      </template>

      <hui-button type="submit" block :loading="loading" :disabled="!canSubmit">Xác nhận và đăng nhập</hui-button>
      <div class="auth-links">
        <RouterLink to="/dang-nhap">Hủy</RouterLink>
        <RouterLink v-if="info.canUsePassword" to="/quen-mat-khau">Quên mật khẩu?</RouterLink>
      </div>
    </form>
  </div>
</template>

<script setup>
import { computed, onBeforeUnmount, ref } from 'vue';
import { useRouter } from 'vue-router';

import { ssoApi } from '../../api';
import { useAuthStore } from '../../stores/auth';
import { useSubmit } from '../../utils/form';
import { PROVIDER_LABELS, parseLinkFragment } from '../../utils/sso';

/** Lỗi khiến yêu cầu liên kết không dùng được nữa → phải đăng nhập lại bằng Google / Microsoft. */
const FATAL = new Set(['SSO_LINK_EXPIRED', 'TOO_MANY_ATTEMPTS', 'ACCOUNT_LOCKED']);

const router = useRouter();
const auth = useAuthStore();

// Đọc token ở phần # rồi xóa khỏi thanh địa chỉ (không để lại trong lịch sử trình duyệt).
const info = parseLinkFragment(window.location.hash);
if (window.location.hash) window.history.replaceState(window.history.state, '', window.location.pathname);

const providerLabel = computed(() => PROVIDER_LABELS[info?.provider] || 'Google / Microsoft');
const methodOptions = [
  { value: 'password', label: 'Mật khẩu' },
  { value: 'code', label: 'Mã qua email' },
];
const method = ref(info?.canUsePassword ? 'password' : 'code');
const password = ref('');
const code = ref('');
const codeSent = ref(false);
const sending = ref(false);
const cooldown = ref(0);
const ended = ref(false);
const { loading, error, fields, run } = useSubmit();

const canSubmit = computed(() => (method.value === 'password' ? password.value.length > 0 : code.value.length === 6));

const timer = setInterval(() => {
  if (cooldown.value > 0) cooldown.value -= 1;
}, 1000);
onBeforeUnmount(() => clearInterval(timer));

function setMethod(value) {
  method.value = value;
  error.value = '';
}

function handleError(e) {
  if (FATAL.has(e?.code)) ended.value = true;
}

async function sendCode() {
  sending.value = true;
  error.value = '';
  try {
    await ssoApi.sendLinkCode(info.token);
    codeSent.value = true;
    cooldown.value = 60;
  } catch (e) {
    error.value = e.message;
    handleError(e);
  } finally {
    sending.value = false;
  }
}

function onComplete(e) {
  code.value = e.detail.value;
  submit();
}

async function submit() {
  if (loading.value || !canSubmit.value) return;
  const body = method.value === 'password' ? { token: info.token, password: password.value } : { token: info.token, code: code.value };
  const res = await run(() => ssoApi.confirmLink(body));
  if (res.ok) {
    auth.applySession(res.data);
    router.replace(info.next || { name: 'home' });
  } else {
    handleError(res.error);
  }
}
</script>

<style scoped>
.field-error {
  margin: -6px 0 0;
  color: var(--hui-danger, #d4380d);
}
</style>

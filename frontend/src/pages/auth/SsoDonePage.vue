<template>
  <div class="auth-form">
    <hui-result v-if="failed" status="error" title="Không đăng nhập được" sub-title="Phiên đăng nhập không còn hiệu lực. Vui lòng thử lại.">
      <hui-button slot="extra" @hui-click="router.replace({ name: 'login' })">Về trang đăng nhập</hui-button>
    </hui-result>
    <div v-else class="waiting">
      <hui-spinner />
      <p class="muted">Đang đăng nhập…</p>
    </div>
  </div>
</template>

<script setup>
/**
 * Callback SSO chuyển về đây sau khi đã đặt cookie refresh. Lúc mở trang, app đã tự khôi phục phiên
 * từ cookie (main.js); trang này chỉ chuyển tiếp tới trang cần đến.
 */
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { useAuthStore } from '../../stores/auth';
import { safeNext } from '../../utils/sso';

const auth = useAuthStore();
const route = useRoute();
const router = useRouter();
const failed = ref(false);

onMounted(async () => {
  if (!auth.isLoggedIn) await auth.restore();
  if (!auth.isLoggedIn) {
    failed.value = true;
    return;
  }
  const next = safeNext(route.query.next);
  if (next) router.replace(next);
  // Tài khoản vừa tạo từ SSO: đưa tới Hồ sơ để xem thông tin và đặt mật khẩu nếu muốn.
  else if (route.query.new === '1') router.replace({ name: 'profile', query: { moi: '1' } });
  else router.replace({ name: 'home' });
});
</script>

<style scoped>
.waiting {
  display: grid;
  justify-items: center;
  gap: 12px;
  padding: 24px 0;
}
</style>

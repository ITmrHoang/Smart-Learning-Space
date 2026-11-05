<template>
  <div class="page">
    <div class="page-header">
      <h1>Hồ sơ của tôi</h1>
    </div>
    <hui-alert
      v-if="route.query.moi === '1'"
      severity="success"
      description="Đã tạo tài khoản. Username được gợi ý từ email; bạn có thể đặt mật khẩu bên dưới để đăng nhập bằng username hoặc email."
    />

    <section class="panel">
      <dl v-if="auth.user" class="info">
        <dt>Họ và tên</dt>
        <dd>{{ auth.user.fullName }}</dd>
        <dt>Username</dt>
        <dd>{{ auth.user.username }}</dd>
        <dt>Email</dt>
        <dd>{{ auth.user.email }}</dd>
        <dt>Vai trò</dt>
        <dd>
          <hui-tag :color="ROLE_COLORS[auth.user.role]">{{ ROLE_LABELS[auth.user.role] }}</hui-tag>
        </dd>
        <dt>Lần đăng nhập trước</dt>
        <dd>{{ formatDateTime(auth.user.lastLoginAt) }}</dd>
      </dl>
    </section>

    <section v-if="auth.user && !auth.user.hasPassword" class="panel">
      <h2>Đặt mật khẩu</h2>
      <p class="muted small">Tài khoản đang đăng nhập bằng Google / Microsoft. Đặt mật khẩu để đăng nhập được cả bằng username hoặc email.</p>
      <form class="form-grid narrow" novalidate @submit.prevent="setPassword">
        <hui-alert v-if="error" severity="error" :description="error" />
        <hui-input
          v-model="form.newPassword"
          type="password"
          label="Mật khẩu mới"
          hint="Ít nhất 8 ký tự, có chữ và số"
          autocomplete="new-password"
          :error="fields.newPassword"
        />
        <div class="form-actions">
          <hui-button type="submit" :loading="loading">Đặt mật khẩu</hui-button>
        </div>
      </form>
    </section>

    <section v-else class="panel">
      <h2>Đổi mật khẩu</h2>
      <form class="form-grid narrow" novalidate @submit.prevent="changePassword">
        <hui-alert v-if="error" severity="error" :description="error" />
        <hui-alert v-if="changed" severity="success" :description="changed" />
        <hui-input
          v-model="form.currentPassword"
          type="password"
          label="Mật khẩu hiện tại"
          autocomplete="current-password"
          :error="fields.currentPassword"
        />
        <hui-input
          v-model="form.newPassword"
          type="password"
          label="Mật khẩu mới"
          hint="Ít nhất 8 ký tự, có chữ và số"
          autocomplete="new-password"
          :error="fields.newPassword"
        />
        <div class="form-actions">
          <hui-button type="submit" :loading="loading">Đổi mật khẩu</hui-button>
        </div>
      </form>
    </section>

    <LinkedAccounts />

    <section class="panel">
      <h2>Phiên đăng nhập</h2>
      <p class="muted">
        Mỗi tài khoản đăng nhập cùng lúc tối đa 2 thiết bị; đăng nhập thiết bị thứ ba thì thiết bị lâu không dùng nhất bị đăng xuất.
        Nút dưới đây đăng xuất khỏi mọi thiết bị, kể cả thiết bị đang dùng.
      </p>
      <hui-button variant="danger" :loading="loggingOutAll" @hui-click="logoutAll">Đăng xuất mọi thiết bị</hui-button>
    </section>
  </div>
</template>

<script setup>
import { reactive, ref } from 'vue';
import { useRoute } from 'vue-router';

import { authApi } from '../api';
import LinkedAccounts from '../components/LinkedAccounts.vue';
import { useAuthStore } from '../stores/auth';
import { formatDateTime, useSubmit } from '../utils/form';
import { ROLE_COLORS, ROLE_LABELS } from '../utils/permissions';

const auth = useAuthStore();
const route = useRoute();
const form = reactive({ currentPassword: '', newPassword: '' });
const changed = ref('');
const loggingOutAll = ref(false);
const { loading, error, fields, run } = useSubmit();

async function changePassword() {
  changed.value = '';
  const res = await run(() => authApi.changePassword(form.currentPassword, form.newPassword));
  if (res.ok) {
    changed.value = 'Đã đổi mật khẩu. Các thiết bị khác đã bị đăng xuất.';
    form.currentPassword = '';
    form.newPassword = '';
  }
}

async function setPassword() {
  changed.value = '';
  const res = await run(() => authApi.setPassword(form.newPassword));
  if (res.ok) {
    form.newPassword = '';
    await auth.reloadMe();
    changed.value = 'Đã đặt mật khẩu. Từ giờ bạn đăng nhập được bằng username hoặc email. Các thiết bị khác đã bị đăng xuất.';
  }
}

async function logoutAll() {
  loggingOutAll.value = true;
  try {
    await auth.logoutAll();
  } finally {
    loggingOutAll.value = false;
  }
}
</script>

<style scoped>
.narrow {
  max-width: 420px;
}
</style>

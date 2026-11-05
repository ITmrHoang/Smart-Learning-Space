<template>
  <div class="page">
    <div class="page-header">
      <h1>Hồ sơ của tôi</h1>
    </div>

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

    <section class="panel">
      <h2>Đổi mật khẩu</h2>
      <form class="form-grid narrow" novalidate @submit.prevent="changePassword">
        <hui-alert v-if="error" severity="error" :description="error" />
        <hui-alert v-if="changed" severity="success" description="Đã đổi mật khẩu. Các thiết bị khác đã bị đăng xuất." />
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

    <section class="panel">
      <h2>Phiên đăng nhập</h2>
      <p class="muted">Đăng xuất khỏi mọi thiết bị, kể cả thiết bị đang dùng.</p>
      <hui-button variant="danger" :loading="loggingOutAll" @hui-click="logoutAll">Đăng xuất mọi thiết bị</hui-button>
    </section>
  </div>
</template>

<script setup>
import { reactive, ref } from 'vue';

import { authApi } from '../api';
import { useAuthStore } from '../stores/auth';
import { formatDateTime, useSubmit } from '../utils/form';
import { ROLE_COLORS, ROLE_LABELS } from '../utils/permissions';

const auth = useAuthStore();
const form = reactive({ currentPassword: '', newPassword: '' });
const changed = ref(false);
const loggingOutAll = ref(false);
const { loading, error, fields, run } = useSubmit();

async function changePassword() {
  changed.value = false;
  const res = await run(() => authApi.changePassword(form.currentPassword, form.newPassword));
  if (res.ok) {
    changed.value = true;
    form.currentPassword = '';
    form.newPassword = '';
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

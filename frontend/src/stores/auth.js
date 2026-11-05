import { defineStore } from 'pinia';
import { computed, ref } from 'vue';

import { authApi } from '../api';
import { onAuthEvents, refreshSession, setAccessToken } from '../api/http';

export const useAuthStore = defineStore('auth', () => {
  /** Người dùng hiện tại (MeResponse: thông tin + permissions) */
  const user = ref(null);
  /** Đã thử khôi phục phiên lúc mở trang xong chưa */
  const ready = ref(false);

  const isLoggedIn = computed(() => user.value !== null);
  const permissions = computed(() => user.value?.permissions ?? []);
  const can = (permission) => permissions.value.includes(permission);

  function applySession(session) {
    setAccessToken(session.accessToken);
    user.value = session.user;
  }

  function clear() {
    setAccessToken(null);
    user.value = null;
  }

  // Refresh tự động (trong api/http.js) cập nhật lại người dùng; refresh thất bại → xóa phiên.
  onAuthEvents({
    session: (session) => {
      user.value = session.user;
    },
    expired: () => clear(),
  });

  /** Mở trang: dùng cookie refresh (nếu có) để khôi phục phiên đăng nhập. */
  async function restore() {
    try {
      applySession(await refreshSession());
    } catch {
      clear();
    } finally {
      ready.value = true;
    }
  }

  async function login(loginName, password) {
    applySession(await authApi.login(loginName, password));
  }

  async function verifyEmail(email, code) {
    applySession(await authApi.verifyEmail(email, code));
  }

  async function logout() {
    try {
      await authApi.logout();
    } finally {
      clear();
    }
  }

  async function logoutAll() {
    try {
      await authApi.logoutAll();
    } finally {
      clear();
    }
  }

  async function reloadMe() {
    user.value = await authApi.me();
  }

  return { user, ready, isLoggedIn, permissions, can, applySession, clear, restore, login, verifyEmail, logout, logoutAll, reloadMe };
});

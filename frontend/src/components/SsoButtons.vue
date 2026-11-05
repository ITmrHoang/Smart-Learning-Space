<template>
  <div v-if="providers.length" class="sso">
    <hui-divider plain spacing="sm">hoặc</hui-divider>
    <hui-button
      v-for="p in providers"
      :key="p"
      variant="outline"
      block
      :loading="busy === p"
      :disabled="busy !== '' && busy !== p"
      @hui-click="go(p)"
    >
      <span slot="icon" class="logo" aria-hidden="true">
        <svg v-if="p === 'google'" viewBox="0 0 48 48" width="18" height="18">
          <path fill="#EA4335" d="M24 9.5c3.5 0 6.6 1.2 9.1 3.6l6.8-6.8C35.8 2.4 30.3 0 24 0 14.6 0 6.6 5.4 2.7 13.3l7.9 6.1C12.5 13.6 17.8 9.5 24 9.5z" />
          <path fill="#4285F4" d="M46.1 24.5c0-1.6-.1-3.1-.4-4.5H24v9h12.4c-.5 2.9-2.2 5.3-4.6 6.9l7.4 5.8c4.3-4 6.9-9.9 6.9-17.2z" />
          <path fill="#FBBC05" d="M10.6 28.6c-.5-1.4-.8-3-.8-4.6s.3-3.2.8-4.6l-7.9-6.1C1 16.6 0 20.2 0 24s1 7.4 2.7 10.7l7.9-6.1z" />
          <path fill="#34A853" d="M24 48c6.5 0 11.9-2.1 15.9-5.8l-7.4-5.8c-2.1 1.4-4.8 2.3-8.5 2.3-6.2 0-11.5-4.1-13.4-9.8l-7.9 6.1C6.6 42.6 14.6 48 24 48z" />
        </svg>
        <svg v-else viewBox="0 0 21 21" width="16" height="16">
          <rect x="1" y="1" width="9" height="9" fill="#F25022" />
          <rect x="11" y="1" width="9" height="9" fill="#7FBA00" />
          <rect x="1" y="11" width="9" height="9" fill="#00A4EF" />
          <rect x="11" y="11" width="9" height="9" fill="#FFB900" />
        </svg>
      </span>
      {{ action }} với {{ PROVIDER_LABELS[p] }}
    </hui-button>
    <hui-alert v-if="error" severity="error" :description="error" />
  </div>
</template>

<script setup>
/** Nút "Đăng nhập với Google / Microsoft". Chỉ hiện nhà cung cấp mà backend đã cấu hình. */
import { onMounted, ref } from 'vue';

import { ssoApi } from '../api';
import { PROVIDER_LABELS, redirectToProvider } from '../utils/sso';

const props = defineProps({
  /** Chữ trên nút: "Đăng nhập" / "Đăng ký" */
  action: { type: String, default: 'Đăng nhập' },
  /** Trang quay về sau khi đăng nhập */
  next: { type: String, default: undefined },
});

const providers = ref([]);
const busy = ref('');
const error = ref('');

onMounted(async () => {
  try {
    providers.value = (await ssoApi.providers()).providers;
  } catch {
    providers.value = [];
  }
});

async function go(provider) {
  busy.value = provider;
  error.value = '';
  try {
    await redirectToProvider(provider, { next: props.next });
  } catch (e) {
    error.value = e.message;
    busy.value = '';
  }
}
</script>

<style scoped>
.sso {
  display: grid;
  gap: 10px;
}
.logo {
  display: inline-flex;
}
</style>

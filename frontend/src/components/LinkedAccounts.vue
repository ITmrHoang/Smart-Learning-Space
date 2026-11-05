<template>
  <section class="panel">
    <h2>Tài khoản liên kết</h2>
    <p class="muted small">Liên kết Google hoặc Microsoft để đăng nhập nhanh mà không cần mật khẩu.</p>
    <hui-alert v-if="notice" severity="success" :description="notice" />
    <hui-alert v-if="error" severity="error" :description="error" />

    <ul class="providers">
      <li v-for="p in rows" :key="p.provider" class="provider">
        <div class="name">
          <strong>{{ PROVIDER_LABELS[p.provider] }}</strong>
          <span v-if="p.identity" class="muted small">
            {{ p.identity.email || 'Đã liên kết' }} · liên kết {{ formatDateTime(p.identity.linkedAt) }}
          </span>
          <span v-else class="muted small">Chưa liên kết</span>
        </div>
        <hui-button
          v-if="p.identity"
          variant="outline"
          size="sm"
          :disabled="busy !== ''"
          @hui-click="askUnlink(p.provider)"
        >
          Gỡ liên kết
        </hui-button>
        <hui-button
          v-else-if="p.enabled"
          size="sm"
          :loading="busy === p.provider"
          :disabled="busy !== '' && busy !== p.provider"
          @hui-click="link(p.provider)"
        >
          Liên kết
        </hui-button>
        <span v-else class="muted small">Chưa bật</span>
      </li>
    </ul>

    <ConfirmModal
      v-model:open="confirm.open"
      title="Gỡ liên kết"
      :message="confirm.message"
      ok-text="Gỡ liên kết"
      danger
      :loading="confirm.loading"
      @confirm="unlink"
    />
  </section>
</template>

<script setup>
/** Mục "Tài khoản liên kết" trong Hồ sơ: xem / liên kết thêm / gỡ Google, Microsoft. */
import { computed, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { ssoApi } from '../api';
import { formatDateTime } from '../utils/form';
import { PROVIDERS, PROVIDER_LABELS, redirectToProvider, ssoErrorMessage } from '../utils/sso';
import ConfirmModal from './ConfirmModal.vue';

const route = useRoute();
const router = useRouter();
const identities = ref([]);
const enabled = ref([]);
const busy = ref('');
const notice = ref('');
const error = ref('');
const confirm = reactive({ open: false, provider: '', message: '', loading: false });

const rows = computed(() =>
  PROVIDERS.map((provider) => ({
    provider,
    identity: identities.value.find((i) => i.provider === provider),
    enabled: enabled.value.includes(provider),
  })),
);

async function load() {
  const [ids, providers] = await Promise.all([ssoApi.identities(), ssoApi.providers()]);
  identities.value = ids;
  enabled.value = providers.providers;
}

onMounted(async () => {
  // Kết quả do callback gắn lên URL (?ssoLinked=google / ?ssoError=…): hiện thông báo rồi xóa khỏi URL.
  const { ssoLinked, ssoError, ...rest } = route.query;
  if (typeof ssoLinked === 'string') notice.value = `Đã liên kết tài khoản ${PROVIDER_LABELS[ssoLinked] || ssoLinked}.`;
  if (typeof ssoError === 'string') error.value = ssoErrorMessage(ssoError);
  if (ssoLinked || ssoError) router.replace({ query: rest });
  try {
    await load();
  } catch (e) {
    error.value = e.message;
  }
});

async function link(provider) {
  busy.value = provider;
  error.value = '';
  try {
    await redirectToProvider(provider, { mode: 'link' });
  } catch (e) {
    error.value = e.message;
    busy.value = '';
  }
}

function askUnlink(provider) {
  Object.assign(confirm, {
    open: true,
    provider,
    loading: false,
    message: `Gỡ liên kết ${PROVIDER_LABELS[provider]}? Bạn sẽ không đăng nhập bằng ${PROVIDER_LABELS[provider]} được nữa và các thiết bị khác sẽ bị đăng xuất.`,
  });
}

async function unlink() {
  confirm.loading = true;
  error.value = '';
  notice.value = '';
  try {
    await ssoApi.unlink(confirm.provider);
    notice.value = `Đã gỡ liên kết ${PROVIDER_LABELS[confirm.provider]}.`;
    confirm.open = false;
    await load();
  } catch (e) {
    error.value = e.message;
    confirm.open = false;
  } finally {
    confirm.loading = false;
  }
}
</script>

<style scoped>
.providers {
  list-style: none;
  margin: 12px 0 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.provider {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-lg, 10px);
}
.name {
  display: grid;
  gap: 2px;
  min-width: 0;
}
</style>

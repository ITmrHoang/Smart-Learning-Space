<template>
  <hui-modal :open="open" :title="user ? `Tài khoản liên kết — ${user.username}` : 'Tài khoản liên kết'" no-footer @hui-close="close">
    <hui-alert v-if="error" severity="error" :description="error" />
    <hui-spinner v-if="loading" />
    <p v-else-if="!items.length" class="muted">Chưa liên kết Google / Microsoft.</p>
    <ul v-else class="items">
      <li v-for="i in items" :key="i.provider" class="item">
        <div class="info">
          <strong>{{ PROVIDER_LABELS[i.provider] || i.provider }}</strong>
          <span class="muted small">{{ i.email || '—' }}</span>
          <span class="muted small">
            Liên kết {{ formatDateTime(i.linkedAt) }} · đăng nhập gần nhất {{ formatDateTime(i.lastLoginAt) }}
          </span>
        </div>
        <hui-button
          v-if="canManage"
          variant="danger"
          size="sm"
          :loading="removing === i.provider"
          :disabled="removing !== '' && removing !== i.provider"
          @hui-click="unlink(i.provider)"
        >
          Gỡ liên kết
        </hui-button>
      </li>
    </ul>
    <p v-if="canManage && items.length" class="muted small">Gỡ liên kết sẽ đăng xuất người dùng khỏi mọi thiết bị.</p>
  </hui-modal>
</template>

<script setup>
/** Admin xem / gỡ liên kết Google, Microsoft của một người dùng. */
import { message } from '@itmrhoang/h-ui/hui-message';
import { ref, watch } from 'vue';

import { adminApi } from '../api';
import { formatDateTime } from '../utils/form';
import { PROVIDER_LABELS } from '../utils/sso';

const props = defineProps({
  open: { type: Boolean, default: false },
  user: { type: Object, default: null },
  canManage: { type: Boolean, default: false },
});
const emit = defineEmits(['update:open']);

const items = ref([]);
const loading = ref(false);
const removing = ref('');
const error = ref('');

async function load() {
  loading.value = true;
  error.value = '';
  try {
    items.value = await adminApi.userIdentities(props.user.id);
  } catch (e) {
    error.value = e.message;
    items.value = [];
  } finally {
    loading.value = false;
  }
}

watch(
  () => [props.open, props.user?.id],
  ([open]) => {
    if (open && props.user) load();
  },
);

async function unlink(provider) {
  removing.value = provider;
  error.value = '';
  try {
    await adminApi.unlinkIdentity(props.user.id, provider);
    message.success(`Đã gỡ liên kết ${PROVIDER_LABELS[provider] || provider}.`);
    await load();
  } catch (e) {
    error.value = e.message;
  } finally {
    removing.value = '';
  }
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.items {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-lg, 10px);
}
.info {
  display: grid;
  gap: 2px;
  min-width: 0;
}
</style>

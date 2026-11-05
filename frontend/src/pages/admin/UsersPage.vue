<template>
  <div class="page">
    <div class="page-header">
      <h1>Người dùng</h1>
      <hui-button v-if="canManage" @hui-click="openForm(null)">Thêm người dùng</hui-button>
    </div>

    <section class="panel toolbar">
      <hui-input
        class="grow"
        type="search"
        label="Tìm kiếm"
        placeholder="Username, email hoặc họ tên"
        clearable
        @hui-search="list.setFilter('q', $event.detail.value)"
        @clear="list.setFilter('q', '')"
      />
      <hui-select
        label="Vai trò"
        placeholder="Tất cả"
        allow-clear
        :options="roleOptions"
        @change="list.setFilter('role', $event.detail.value)"
      />
      <hui-select
        label="Trạng thái"
        placeholder="Tất cả"
        allow-clear
        :options="statusOptions"
        @change="list.setFilter('status', $event.detail.value)"
      />
    </section>

    <section class="panel">
      <hui-alert v-if="list.error.value" severity="error" :description="list.error.value" />
      <hui-table
        row-key="id"
        pagination="none"
        hover
        :columns="columns"
        :data="list.items.value"
        scroll-x="1040px"
        :loading="list.loading.value"
        empty-text="Không có người dùng nào"
        @hui-action="onAction"
      />
      <hui-pagination
        class="pager"
        :total="list.total.value"
        :current="list.page.value"
        :pageSize="list.pageSize.value"
        show-total
        show-size-changer
        hide-on-single-page
        @change="list.onPage($event.detail)"
      />
    </section>

    <UserFormModal v-model:open="formOpen" :user="editing" @saved="onSaved" />
    <UserIdentitiesModal v-model:open="identitiesOpen" :user="identitiesUser" :canManage="canManage" />
    <ConfirmModal
      v-model:open="confirm.open"
      :title="confirm.title"
      :message="confirm.message"
      :okText="confirm.okText"
      :danger="confirm.danger"
      :loading="confirm.loading"
      @confirm="runConfirm"
    />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';

import { adminApi } from '../../api';
import ConfirmModal from '../../components/ConfirmModal.vue';
import UserFormModal from '../../components/UserFormModal.vue';
import UserIdentitiesModal from '../../components/UserIdentitiesModal.vue';
import { useAuthStore } from '../../stores/auth';
import { usePagedList } from '../../utils/list';
import { P, ROLE_COLORS, ROLE_LABELS, STATUS_COLORS, STATUS_LABELS } from '../../utils/permissions';

const auth = useAuthStore();
const canManage = computed(() => auth.can(P.USERS_MANAGE));
const list = usePagedList((q) => adminApi.users(q), { q: '', role: '', status: '' });

const toOptions = (labels) => Object.entries(labels).map(([value, label]) => ({ value, label }));
const roleOptions = toOptions(ROLE_LABELS);
const statusOptions = toOptions(STATUS_LABELS);

const columns = computed(() => [
  { key: 'username', title: 'Username' },
  { key: 'fullName', title: 'Họ và tên' },
  { key: 'email', title: 'Email', ellipsis: true },
  { key: 'role', title: 'Vai trò', type: 'tag', labels: ROLE_LABELS, colors: ROLE_COLORS },
  { key: 'status', title: 'Trạng thái', type: 'tag', labels: STATUS_LABELS, colors: STATUS_COLORS },
  { key: 'lastLoginAt', title: 'Đăng nhập gần nhất', type: 'datetime', width: '170px' },
  ...(canManage.value
    ? [
        {
          key: 'actions',
          title: 'Thao tác',
          type: 'actions',
          actions: [
            { key: 'edit', label: 'Sửa' },
            { key: 'identities', label: 'Liên kết SSO' },
            { key: 'reset', label: 'Gửi đặt lại MK', disabled: (r) => r.status === 'locked' },
            { key: 'unlock', label: 'Mở khóa', disabled: (r) => r.status !== 'locked' },
            { key: 'lock', label: 'Khóa', danger: true, disabled: (r) => r.status === 'locked' || r.id === auth.user?.id },
          ],
        },
      ]
    : []),
]);

const formOpen = ref(false);
const editing = ref(null);
function openForm(user) {
  editing.value = user;
  formOpen.value = true;
}
function onSaved() {
  message.success('Đã lưu người dùng.');
  list.load();
}

const identitiesOpen = ref(false);
const identitiesUser = ref(null);

const confirm = reactive({ open: false, title: '', message: '', okText: '', danger: false, loading: false, action: null });

function ask({ title, message: text, okText, danger = false, action }) {
  Object.assign(confirm, { open: true, title, message: text, okText, danger, loading: false, action });
}

async function runConfirm() {
  confirm.loading = true;
  try {
    await confirm.action();
    confirm.open = false;
    list.load();
  } catch (e) {
    message.error(e.message);
  } finally {
    confirm.loading = false;
  }
}

function onAction({ detail: { action, row } }) {
  if (action === 'edit') return openForm(row);
  if (action === 'identities') {
    identitiesUser.value = row;
    identitiesOpen.value = true;
    return;
  }
  if (action === 'lock') {
    return ask({
      title: 'Khóa tài khoản',
      message: `Khóa "${row.username}"? Người này bị đăng xuất khỏi mọi thiết bị và không đăng nhập được nữa.`,
      okText: 'Khóa',
      danger: true,
      action: async () => {
        await adminApi.lockUser(row.id);
        message.success(`Đã khóa ${row.username}.`);
      },
    });
  }
  if (action === 'unlock') {
    return ask({
      title: 'Mở khóa tài khoản',
      message: `Mở khóa "${row.username}"?`,
      okText: 'Mở khóa',
      action: async () => {
        await adminApi.unlockUser(row.id);
        message.success(`Đã mở khóa ${row.username}.`);
      },
    });
  }
  if (action === 'reset') {
    return ask({
      title: 'Gửi link đặt lại mật khẩu',
      message: `Gửi email đặt lại mật khẩu tới ${row.email}?`,
      okText: 'Gửi',
      action: async () => {
        await adminApi.sendPasswordReset(row.id);
        message.success('Đã gửi email.');
      },
    });
  }
}

onMounted(() => list.load());
</script>

<style scoped>
.pager {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

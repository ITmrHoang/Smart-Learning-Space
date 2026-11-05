<template>
  <hui-modal
    :open="open"
    :title="user ? 'Sửa người dùng' : 'Thêm người dùng'"
    ok-text="Lưu"
    cancel-text="Hủy"
    :confirmLoading="loading"
    @hui-ok="onOk"
    @hui-close="close"
  >
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input v-model="form.fullName" label="Họ và tên" :error="fields.fullName" required />
      <hui-input
        v-if="!user"
        v-model="form.username"
        label="Username"
        hint="3–32 ký tự: chữ thường không dấu, số, dấu chấm, gạch dưới"
        :error="fields.username"
        required
      />
      <hui-input v-model="form.email" type="email" label="Email" :error="fields.email" required />
      <hui-input
        v-if="!user"
        v-model="form.password"
        type="password"
        label="Mật khẩu ban đầu"
        hint="Ít nhất 8 ký tự, có chữ và số. Người dùng nên đổi sau khi đăng nhập."
        autocomplete="new-password"
        :error="fields.password"
        required
      />
      <hui-select
        label="Vai trò"
        :options="roleOptions"
        :value="form.role"
        :error="fields.role"
        :hint="isSelf ? 'Bạn không thể tự đổi vai trò của mình.' : ''"
        :disabled="isSelf"
        @change="form.role = $event.detail.value"
      />
    </form>
  </hui-modal>
</template>

<script setup>
import { computed, reactive, watch } from 'vue';

import { adminApi } from '../api';
import { useAuthStore } from '../stores/auth';
import { useSubmit } from '../utils/form';
import { ROLE_LABELS } from '../utils/permissions';

const props = defineProps({
  open: { type: Boolean, default: false },
  /** null = tạo mới */
  user: { type: Object, default: null },
});
const emit = defineEmits(['update:open', 'saved']);

const auth = useAuthStore();
const roleOptions = Object.entries(ROLE_LABELS).map(([value, label]) => ({ value, label }));
const form = reactive({ fullName: '', username: '', email: '', password: '', role: 'student' });
const isSelf = computed(() => props.user?.id === auth.user?.id);
const { loading, error, fields, run, reset } = useSubmit();

watch(
  () => [props.open, props.user],
  () => {
    if (!props.open) return;
    reset();
    const u = props.user;
    Object.assign(form, {
      fullName: u?.fullName ?? '',
      username: '',
      email: u?.email ?? '',
      password: '',
      role: u?.role ?? 'student',
    });
  },
  { immediate: true },
);

async function save() {
  const res = await run(() => {
    if (!props.user) {
      return adminApi.createUser({
        fullName: form.fullName.trim(),
        username: form.username.trim().toLowerCase(),
        email: form.email.trim(),
        password: form.password,
        role: form.role,
      });
    }
    const changes = {};
    if (form.fullName.trim() !== props.user.fullName) changes.fullName = form.fullName.trim();
    if (form.email.trim().toLowerCase() !== props.user.email) changes.email = form.email.trim();
    if (form.role !== props.user.role) changes.role = form.role;
    return adminApi.updateUser(props.user.id, changes);
  });
  if (res.ok) {
    emit('saved', res.data);
    close();
  }
}

function onOk(e) {
  e.preventDefault();
  save();
}

function close() {
  emit('update:open', false);
}
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1>Chào {{ auth.user?.fullName }}</h1>
    </div>

    <section class="panel">
      <p>
        Bạn đang đăng nhập với vai trò <strong>{{ ROLE_LABELS[auth.user?.role] }}</strong>.
        Vào <RouterLink to="/lop-hoc-cua-toi">Lớp học của tôi</RouterLink> để học tiếp, hoặc <RouterLink to="/kham-pha">tìm lớp học</RouterLink> mới.
      </p>
      <ul class="shortcuts">
        <li v-for="item in shortcuts" :key="item.to">
          <RouterLink :to="item.to">{{ item.label }}</RouterLink>
        </li>
      </ul>
    </section>

    <section v-if="auth.can(P.TEACHER_SUBMIT) && latestApplication" class="panel">
      <h2>Hồ sơ giáo viên của bạn</h2>
      <p>
        Trạng thái:
        <hui-tag :color="APP_STATUS_COLORS[latestApplication.status]">{{ APP_STATUS_LABELS[latestApplication.status] }}</hui-tag>
      </p>
      <p v-if="latestApplication.rejectReason" class="muted">Lý do: {{ latestApplication.rejectReason }}</p>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue';

import { teacherApi } from '../api';
import { useAuthStore } from '../stores/auth';
import { APP_STATUS_COLORS, APP_STATUS_LABELS } from '../utils/labels';
import { P, ROLE_LABELS, visibleMenu } from '../utils/permissions';

const auth = useAuthStore();
const applications = ref([]);
const latestApplication = computed(() => applications.value[0] ?? null);
const shortcuts = computed(() =>
  visibleMenu(auth.permissions)
    .flatMap((g) => g.items)
    .filter((i) => i.to !== '/'),
);

onMounted(async () => {
  if (auth.can(P.TEACHER_SUBMIT)) {
    try {
      applications.value = await teacherApi.mine();
    } catch {
      applications.value = [];
    }
  }
});
</script>

<style scoped>
.shortcuts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 20px;
  padding: 0;
  margin-bottom: 0;
  list-style: none;
}
</style>

<template>
  <div class="page">
    <div class="page-header">
      <h1>Đăng ký làm giáo viên</h1>
    </div>

    <section v-if="pending" class="panel">
      <hui-alert severity="info" title="Hồ sơ đang chờ duyệt" description="Quản trị viên sẽ xem xét hồ sơ và gửi kết quả qua email." />
    </section>

    <section v-else class="panel">
      <p class="muted">Chọn các môn bạn muốn dạy và giới thiệu bản thân. Hồ sơ được duyệt thì tài khoản của bạn trở thành tài khoản giáo viên.</p>
      <form class="form-grid narrow" novalidate @submit.prevent="submit">
        <hui-alert v-if="error" severity="error" :description="error" />
        <hui-select
          label="Môn muốn dạy"
          mode="multiple"
          placeholder="Chọn một hoặc nhiều môn"
          :options="subjectOptions"
          :value="form.subjectIds"
          :error="fields.subjectIds"
          searchable
          @change="form.subjectIds = $event.detail.value"
        />
        <hui-select
          label="Lớp muốn dạy (không bắt buộc)"
          mode="multiple"
          placeholder="Chọn lớp"
          :options="gradeOptions"
          :value="form.gradeIds"
          :error="fields.gradeIds"
          @change="form.gradeIds = $event.detail.value"
        />
        <hui-textarea
          :value="form.bio"
          label="Giới thiệu bản thân"
          hint="20–2000 ký tự: trình độ, nơi công tác, phương pháp dạy…"
          :rows="5"
          :error="fields.bio"
          show-count
          @input="form.bio = $event.detail.value"
        />
        <hui-textarea
          :value="form.experience"
          label="Kinh nghiệm (không bắt buộc)"
          :rows="3"
          :error="fields.experience"
          @input="form.experience = $event.detail.value"
        />
        <div class="form-actions">
          <hui-button type="submit" :loading="loading">Gửi hồ sơ</hui-button>
        </div>
      </form>
    </section>

    <section v-if="applications.length" class="panel">
      <h2>Hồ sơ đã gửi</h2>
      <ul class="history">
        <li v-for="app in applications" :key="app.id">
          <hui-tag :color="APP_STATUS_COLORS[app.status]">{{ APP_STATUS_LABELS[app.status] }}</hui-tag>
          <span>{{ formatDateTime(app.createdAt) }}</span>
          <span class="muted">{{ subjectNames(app.subjectIds) }}</span>
          <span v-if="app.rejectReason" class="muted">— Lý do: {{ app.rejectReason }}</span>
        </li>
      </ul>
    </section>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';

import { teacherApi } from '../../api';
import { useMasterData } from '../../utils/masterData';
import { formatDateTime, useSubmit } from '../../utils/form';
import { APP_STATUS_COLORS, APP_STATUS_LABELS } from '../../utils/labels';

const { data: master, load } = useMasterData();
const applications = ref([]);
const form = reactive({ subjectIds: [], gradeIds: [], bio: '', experience: '' });
const { loading, error, fields, run } = useSubmit();

const pending = computed(() => applications.value.some((a) => a.status === 'pending'));
const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ label: s.name, value: s.id })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ label: g.name, value: g.id })));
const subjectNames = (ids) =>
  (master.value?.subjects ?? [])
    .filter((s) => ids.includes(s.id))
    .map((s) => s.name)
    .join(', ');

async function refresh() {
  applications.value = await teacherApi.mine();
}

async function submit() {
  const res = await run(() =>
    teacherApi.submit({ subjectIds: form.subjectIds, gradeIds: form.gradeIds, bio: form.bio.trim(), experience: form.experience.trim() }),
  );
  if (res.ok) {
    message.success('Đã gửi hồ sơ, vui lòng chờ duyệt.');
    await refresh();
  }
}

onMounted(async () => {
  await Promise.all([load(), refresh()]);
});
</script>

<style scoped>
.narrow {
  max-width: 560px;
}
.history {
  display: grid;
  gap: 8px;
  padding: 0;
  margin: 0;
  list-style: none;
}
.history li {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
</style>

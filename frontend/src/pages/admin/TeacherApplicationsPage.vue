<template>
  <div class="page">
    <div class="page-header">
      <h1>Hồ sơ giáo viên</h1>
    </div>

    <section class="panel toolbar">
      <hui-select
        label="Trạng thái"
        :options="statusOptions"
        :value="list.filters.status"
        allow-clear
        placeholder="Tất cả"
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
        :data="rows"
        :loading="list.loading.value"
        empty-text="Không có hồ sơ nào"
        @hui-action="onAction"
      />
      <hui-pagination
        class="pager"
        :total="list.total.value"
        :current="list.page.value"
        :pageSize="list.pageSize.value"
        show-total
        hide-on-single-page
        @change="list.onPage($event.detail)"
      />
    </section>

    <hui-modal :open="detailOpen" title="Chi tiết hồ sơ" no-footer @hui-close="detailOpen = false">
      <dl v-if="selected" class="info">
        <dt>Họ và tên</dt>
        <dd>{{ selected.name }} ({{ selected.username }})</dd>
        <dt>Email</dt>
        <dd>{{ selected.email }}</dd>
        <dt>Môn</dt>
        <dd>{{ selected.subjects }}</dd>
        <dt>Lớp</dt>
        <dd>{{ selected.grades || '—' }}</dd>
        <dt>Giới thiệu</dt>
        <dd class="prewrap">{{ selected.bio }}</dd>
        <dt>Kinh nghiệm</dt>
        <dd class="prewrap">{{ selected.experience || '—' }}</dd>
        <template v-if="selected.rejectReason">
          <dt>Lý do từ chối</dt>
          <dd>{{ selected.rejectReason }}</dd>
        </template>
      </dl>
    </hui-modal>

    <ConfirmModal
      v-model:open="approveOpen"
      title="Duyệt hồ sơ"
      :message="selected ? `Duyệt ${selected.name} làm giáo viên các môn: ${selected.subjects}?` : ''"
      ok-text="Duyệt"
      :loading="working"
      @confirm="approve"
    />

    <hui-modal
      :open="rejectOpen"
      title="Từ chối hồ sơ"
      ok-text="Từ chối"
      ok-type="danger"
      cancel-text="Hủy"
      :confirmLoading="working"
      @hui-ok="onRejectOk"
      @hui-close="rejectOpen = false"
    >
      <div class="form-grid">
        <hui-alert v-if="rejectError" severity="error" :description="rejectError" />
        <hui-textarea
          :value="rejectReason"
          label="Lý do (gửi kèm email cho người đăng ký)"
          :rows="3"
          @input="rejectReason = $event.detail.value"
        />
      </div>
    </hui-modal>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref } from 'vue';

import { adminApi } from '../../api';
import ConfirmModal from '../../components/ConfirmModal.vue';
import { APP_STATUS_COLORS, APP_STATUS_LABELS } from '../../utils/labels';
import { usePagedList } from '../../utils/list';
import { useMasterData } from '../../utils/masterData';

const list = usePagedList((q) => adminApi.teacherApplications(q), { status: 'pending' });
const { data: master, load: loadMaster } = useMasterData();
const statusOptions = Object.entries(APP_STATUS_LABELS).map(([value, label]) => ({ value, label }));

const columns = [
  { key: 'createdAt', title: 'Ngày gửi', type: 'datetime', width: '160px' },
  { key: 'name', title: 'Họ và tên' },
  { key: 'email', title: 'Email', ellipsis: true },
  { key: 'subjects', title: 'Môn', ellipsis: true },
  { key: 'status', title: 'Trạng thái', type: 'tag', labels: APP_STATUS_LABELS, colors: APP_STATUS_COLORS },
  {
    key: 'actions',
    title: 'Thao tác',
    type: 'actions',
    actions: [
      { key: 'view', label: 'Xem' },
      { key: 'approve', label: 'Duyệt', disabled: (r) => r.status !== 'pending' },
      { key: 'reject', label: 'Từ chối', danger: true, disabled: (r) => r.status !== 'pending' },
    ],
  },
];

const namesOf = (items, ids) =>
  (items ?? [])
    .filter((x) => ids.includes(x.id))
    .map((x) => x.name)
    .join(', ');

const rows = computed(() =>
  list.items.value.map((a) => ({
    ...a,
    name: a.applicant.fullName,
    username: a.applicant.username,
    email: a.applicant.email,
    subjects: namesOf(master.value?.subjects, a.subjectIds),
    grades: namesOf(master.value?.grades, a.gradeIds),
  })),
);

const selected = ref(null);
const detailOpen = ref(false);
const approveOpen = ref(false);
const rejectOpen = ref(false);
const rejectReason = ref('');
const rejectError = ref('');
const working = ref(false);

function onAction({ detail: { action, row } }) {
  selected.value = row;
  if (action === 'view') detailOpen.value = true;
  if (action === 'approve') approveOpen.value = true;
  if (action === 'reject') {
    rejectReason.value = '';
    rejectError.value = '';
    rejectOpen.value = true;
  }
}

async function approve() {
  working.value = true;
  try {
    await adminApi.approveApplication(selected.value.id);
    message.success(`Đã duyệt ${selected.value.name}.`);
    approveOpen.value = false;
    list.load();
  } catch (e) {
    message.error(e.message);
  } finally {
    working.value = false;
  }
}

async function onRejectOk(e) {
  e.preventDefault();
  working.value = true;
  rejectError.value = '';
  try {
    await adminApi.rejectApplication(selected.value.id, rejectReason.value.trim());
    message.success('Đã từ chối hồ sơ.');
    rejectOpen.value = false;
    list.load();
  } catch (err) {
    rejectError.value = err.fields?.reason?.[0] ?? err.message;
  } finally {
    working.value = false;
  }
}

onMounted(() => Promise.all([loadMaster(), list.load()]));
</script>

<style scoped>
.pager {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
.prewrap {
  white-space: pre-wrap;
}
</style>

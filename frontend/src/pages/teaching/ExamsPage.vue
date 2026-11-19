<template>
  <div class="page">
    <div class="page-header">
      <h1>Đề ôn tập / kiểm tra</h1>
      <hui-button @hui-click="router.push({ name: 'teach-exam', params: { id: 'moi' } })">Tạo đề</hui-button>
    </div>
    <p class="muted small">
      Đề <b>ôn tập</b>: học sinh làm bao nhiêu lần cũng được, kiểm tra từng câu và xem lời giải. Đề <b>kiểm tra</b>: có thời gian, khung giờ, số lần làm.
      Đề gắn vào lớp học bằng cách thêm vào lộ trình, hoặc bấm <b>Giao nhanh</b> để giao ngay cho lớp / nhóm / học sinh. Gom nhiều đề thành <RouterLink to="/day-hoc/bo-de">bộ đề</RouterLink>.
    </p>
    <section class="panel toolbar">
      <hui-input class="grow" type="search" label="Tìm đề" clearable @hui-search="list.setFilter('q', $event.detail.value)" @clear="list.setFilter('q', '')" />
      <hui-select label="Môn" allow-clear placeholder="Tất cả" :options="subjectOptions" @change="list.setFilter('subjectId', $event.detail.value)" />
      <hui-select label="Loại" allow-clear placeholder="Tất cả" :options="kindOptions" @change="list.setFilter('kind', $event.detail.value)" />
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
        empty-text="Chưa có đề nào"
        @hui-action="onAction"
        @hui-row-click="open($event.detail.row)"
      />
      <hui-pagination class="pager" :total="list.total.value" :current="list.page.value" :pageSize="list.pageSize.value" show-total hide-on-single-page @change="list.onPage($event.detail)" />
    </section>
    <QuickAssignModal v-model:open="assignOpen" kind="exam" :refId="assigning?.id ?? ''" :refTitle="assigning?.title ?? ''" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import QuickAssignModal from '../../components/teaching/QuickAssignModal.vue';
import { usePagedList } from '../../utils/list';
import { useMasterData } from '../../utils/masterData';
import { EXAM_KIND_COLORS, EXAM_KIND_LABELS, toOptions } from '../../utils/teaching';

const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const list = usePagedList((q) => teachingApi.exams(q), { q: '', subjectId: '', kind: '' });
const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const kindOptions = toOptions(EXAM_KIND_LABELS);
const assignOpen = ref(false);
const assigning = ref(null);

const rows = computed(() =>
  list.items.value.map((e) => ({
    ...e,
    subject: master.value?.subjects.find((s) => s.id === e.subjectId)?.name ?? '',
    time: e.timeLimitMinutes ? `${e.timeLimitMinutes} phút` : 'Không giới hạn',
  })),
);
const columns = [
  { key: 'title', title: 'Tên đề' },
  { key: 'kind', title: 'Loại', type: 'tag', labels: EXAM_KIND_LABELS, colors: EXAM_KIND_COLORS, width: '110px' },
  { key: 'subject', title: 'Môn', width: '120px' },
  { key: 'questionCount', title: 'Số câu', width: '90px' },
  { key: 'time', title: 'Thời gian', width: '140px' },
  { key: 'attemptCount', title: 'Lượt làm', width: '100px' },
  { key: 'updatedAt', title: 'Sửa lần cuối', type: 'datetime', width: '170px' },
  {
    key: 'actions',
    title: '',
    type: 'actions',
    actions: [{ key: 'assign', label: 'Giao nhanh' }, { key: 'papers', label: 'Trộn mã' }, { key: 'edit', label: 'Sửa' }, { key: 'archive', label: 'Ẩn', danger: true }],
  },
];

function open(row) {
  router.push({ name: 'teach-exam', params: { id: row.id } });
}

async function onAction({ detail: { action, row } }) {
  if (action === 'edit') return open(row);
  if (action === 'papers') return router.push({ name: 'teach-exam-papers', params: { id: row.id } });
  if (action === 'assign') {
    assigning.value = row;
    assignOpen.value = true;
    return;
  }
  if (action === 'archive' && window.confirm(`Ẩn đề "${row.title}"? Học sinh sẽ không bắt đầu lượt làm mới được.`)) {
    try {
      await teachingApi.archiveExam(row.id);
      message.success('Đã ẩn đề.');
      list.load();
    } catch (e) {
      message.error(e.message);
    }
  }
}

onMounted(() => {
  loadMaster();
  list.load();
});
</script>

<style scoped>
.pager {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

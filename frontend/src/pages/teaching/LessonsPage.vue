<template>
  <div class="page">
    <div class="page-header">
      <h1>Bài học</h1>
      <hui-button @hui-click="router.push({ name: 'teach-lesson', params: { id: 'moi' } })">Soạn bài mới</hui-button>
    </div>
    <p class="muted small">Thư viện bài của bạn. Một bài dùng được ở nhiều lớp học; sửa xong bấm "Xuất bản" thì mọi lớp học đều thấy bản mới.</p>

    <section class="panel toolbar">
      <hui-input class="grow" type="search" label="Tìm bài" placeholder="Tên bài" clearable @hui-search="list.setFilter('q', $event.detail.value)" @clear="list.setFilter('q', '')" />
      <hui-select label="Môn" allow-clear placeholder="Tất cả" :options="subjectOptions" @change="list.setFilter('subjectId', $event.detail.value)" />
      <hui-select label="Trạng thái" allow-clear placeholder="Tất cả" :options="statusOptions" @change="list.setFilter('status', $event.detail.value)" />
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
        empty-text="Chưa có bài học nào"
        @hui-action="onAction"
        @hui-row-click="open($event.detail.row)"
      />
      <hui-pagination class="pager" :total="list.total.value" :current="list.page.value" :pageSize="list.pageSize.value" show-total hide-on-single-page @change="list.onPage($event.detail)" />
    </section>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted } from 'vue';
import { useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import { usePagedList } from '../../utils/list';
import { useMasterData } from '../../utils/masterData';

const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const list = usePagedList((q) => teachingApi.lessons(q), { q: '', subjectId: '', status: '' });

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const statusOptions = [
  { value: 'draft', label: 'Chưa xuất bản' },
  { value: 'published', label: 'Đã xuất bản' },
  { value: 'changed', label: 'Có thay đổi chưa xuất bản' },
];
const STATUS = { draft: 'Bản nháp', published: 'Đã xuất bản', changed: 'Có thay đổi' };
const STATUS_COLOR = { draft: 'warning', published: 'success', changed: 'info' };

const rows = computed(() =>
  list.items.value.map((l) => ({
    ...l,
    subject: master.value?.subjects.find((s) => s.id === l.subjectId)?.name ?? '',
    status: l.publishedVersion == null ? 'draft' : l.hasChanges ? 'changed' : 'published',
    version: l.publishedVersion ? `v${l.publishedVersion}` : '—',
    video: l.hasVideo ? 'Có' : '',
  })),
);

const columns = [
  { key: 'title', title: 'Tên bài' },
  { key: 'subject', title: 'Môn', width: '130px' },
  { key: 'status', title: 'Trạng thái', type: 'tag', labels: STATUS, colors: STATUS_COLOR, width: '150px' },
  { key: 'version', title: 'Phiên bản', width: '100px' },
  { key: 'video', title: 'Video', width: '80px' },
  { key: 'updatedAt', title: 'Sửa lần cuối', type: 'datetime', width: '170px' },
  { key: 'actions', title: '', type: 'actions', actions: [{ key: 'edit', label: 'Sửa' }, { key: 'archive', label: 'Ẩn', danger: true }] },
];

function open(row) {
  router.push({ name: 'teach-lesson', params: { id: row.id } });
}

async function onAction({ detail: { action, row } }) {
  if (action === 'edit') return open(row);
  if (action === 'archive' && window.confirm(`Ẩn bài "${row.title}" khỏi thư viện? Lớp học đang dùng bài vẫn giữ bản đã xuất bản.`)) {
    try {
      await teachingApi.archiveLesson(row.id);
      message.success('Đã ẩn bài.');
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

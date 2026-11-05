<template>
  <div class="page">
    <div class="page-header">
      <h1>Nhật ký thao tác</h1>
    </div>

    <section class="panel toolbar">
      <hui-select
        label="Hành động"
        placeholder="Tất cả"
        allow-clear
        searchable
        :options="actionOptions"
        @change="list.setFilter('action', $event.detail.value)"
      />
    </section>

    <section class="panel">
      <hui-alert v-if="list.error.value" severity="error" :description="list.error.value" />
      <hui-table
        row-key="id"
        pagination="none"
        :columns="columns"
        :data="rows"
        :loading="list.loading.value"
        empty-text="Chưa có thao tác nào"
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
  </div>
</template>

<script setup>
import { computed, onMounted } from 'vue';

import { adminApi } from '../../api';
import { AUDIT_ACTION_LABELS } from '../../utils/labels';
import { usePagedList } from '../../utils/list';

const list = usePagedList((q) => adminApi.auditLogs(q), { action: '' });
const actionOptions = Object.entries(AUDIT_ACTION_LABELS).map(([value, label]) => ({ value, label }));

const columns = [
  { key: 'createdAt', title: 'Thời gian', type: 'datetime', width: '170px' },
  { key: 'actorUsername', title: 'Người thực hiện' },
  { key: 'actionLabel', title: 'Hành động' },
  { key: 'summary', title: 'Chi tiết', ellipsis: true },
  { key: 'ip', title: 'IP' },
];

/** Tóm tắt dữ liệu kèm theo (bỏ trường rỗng) thành một dòng chữ. */
function summarize(data) {
  return Object.entries(data || {})
    .filter(([, v]) => v !== null && v !== undefined)
    .map(([k, v]) => `${k}: ${typeof v === 'object' ? JSON.stringify(v) : v}`)
    .join(' · ');
}

const rows = computed(() =>
  list.items.value.map((r) => ({
    ...r,
    actorUsername: r.actorUsername ?? '—',
    actionLabel: AUDIT_ACTION_LABELS[r.action] ?? r.action,
    summary: summarize(r.data),
  })),
);

onMounted(() => list.load());
</script>

<style scoped>
.pager {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>

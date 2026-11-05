<template>
  <div class="page">
    <div class="page-header">
      <h1>Lớp</h1>
    </div>
    <p class="muted">
      Lớp 1–12 là mục hệ thống, không sửa được. Có thể thêm lớp riêng dùng chung (ví dụ "Ôn thi IELTS", "Đại học").
      Lớp do giáo viên tự thêm nằm ở trang <RouterLink to="/admin/muc-giao-vien-them">Mục giáo viên thêm</RouterLink>.
    </p>

    <section class="panel">
      <form class="toolbar" novalidate @submit.prevent="create">
        <hui-input v-model="newName" class="grow" label="Thêm lớp dùng chung" placeholder="Ví dụ: Ôn thi IELTS" :error="fields.name" />
        <hui-button type="submit" :loading="loading">Thêm</hui-button>
      </form>
      <hui-alert v-if="error" severity="error" :description="error" />
    </section>

    <section class="panel">
      <hui-table
        row-key="id"
        pagination="none"
        hover
        :columns="columns"
        :data="grades"
        :loading="loadingList"
        @hui-action="onAction"
      />
    </section>

    <ItemFormModal v-model:open="editOpen" title="Sửa lớp" :item="editing" :saveFn="saveEdit" @saved="onSaved" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, ref } from 'vue';

import { adminApi, catalogApi } from '../../api';
import ItemFormModal from '../../components/ItemFormModal.vue';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';

const grades = ref([]);
const loadingList = ref(false);
const newName = ref('');
const editOpen = ref(false);
const editing = ref(null);
const { loading, error, fields, run } = useSubmit();
const { load: reloadMasterData } = useMasterData();

const columns = [
  { key: 'name', title: 'Tên lớp' },
  { key: 'kind', title: 'Loại', type: 'tag', labels: { system: 'Hệ thống', custom: 'Lớp riêng' }, colors: { system: 'primary', custom: 'info' } },
  { key: 'sortOrder', title: 'Thứ tự', type: 'number', width: '90px' },
  { key: 'isActive', title: 'Trạng thái', type: 'tag', labels: { true: 'Đang dùng', false: 'Đã ẩn' }, colors: { true: 'success', false: 'secondary' } },
  { key: 'actions', title: 'Thao tác', type: 'actions', actions: [{ key: 'edit', label: 'Sửa', disabled: (r) => r.isSystem }] },
];

async function refresh() {
  loadingList.value = true;
  try {
    const list = await adminApi.grades();
    grades.value = list.map((g) => ({ ...g, kind: g.isSystem ? 'system' : 'custom' }));
  } finally {
    loadingList.value = false;
  }
}

async function create() {
  // Admin gọi API thêm lớp → lớp dùng chung.
  const res = await run(() => catalogApi.createGrade({ name: newName.value.trim() }));
  if (res.ok) {
    newName.value = '';
    message.success(`Đã thêm ${res.data.name}.`);
    await refresh();
    reloadMasterData(true);
  }
}

function onAction({ detail: { action, row } }) {
  if (action === 'edit') {
    editing.value = row;
    editOpen.value = true;
  }
}

const saveEdit = (changes) => adminApi.updateGrade(editing.value.id, changes);

function onSaved() {
  message.success('Đã lưu.');
  refresh();
  reloadMasterData(true);
}

onMounted(refresh);
</script>

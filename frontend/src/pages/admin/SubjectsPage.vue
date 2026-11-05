<template>
  <div class="page">
    <div class="page-header">
      <h1>Môn học</h1>
    </div>

    <section class="panel">
      <form class="toolbar" novalidate @submit.prevent="create">
        <hui-input v-model="newName" class="grow" label="Thêm môn học" placeholder="Ví dụ: Âm nhạc" :error="fields.name" />
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
        :data="subjects"
        :loading="loadingList"
        @hui-action="onAction"
      />
    </section>

    <ItemFormModal v-model:open="editOpen" title="Sửa môn học" :item="editing" :saveFn="saveEdit" @saved="onSaved" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, ref } from 'vue';

import { adminApi } from '../../api';
import ItemFormModal from '../../components/ItemFormModal.vue';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';

const subjects = ref([]);
const loadingList = ref(false);
const newName = ref('');
const editOpen = ref(false);
const editing = ref(null);
const { loading, error, fields, run } = useSubmit();
const { load: reloadMasterData } = useMasterData();

const columns = [
  { key: 'name', title: 'Tên môn' },
  { key: 'slug', title: 'Slug' },
  { key: 'sortOrder', title: 'Thứ tự', type: 'number', width: '90px' },
  { key: 'isActive', title: 'Trạng thái', type: 'tag', labels: { true: 'Đang dùng', false: 'Đã ẩn' }, colors: { true: 'success', false: 'secondary' } },
  { key: 'actions', title: 'Thao tác', type: 'actions', actions: [{ key: 'edit', label: 'Sửa' }] },
];

async function refresh() {
  loadingList.value = true;
  try {
    subjects.value = await adminApi.subjects();
  } finally {
    loadingList.value = false;
  }
}

async function create() {
  const res = await run(() => adminApi.createSubject({ name: newName.value.trim() }));
  if (res.ok) {
    newName.value = '';
    message.success(`Đã thêm môn ${res.data.name}.`);
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

const saveEdit = (changes) => adminApi.updateSubject(editing.value.id, changes);

function onSaved() {
  message.success('Đã lưu.');
  refresh();
  reloadMasterData(true);
}

onMounted(refresh);
</script>

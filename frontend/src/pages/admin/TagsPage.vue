<template>
  <div class="page">
    <div class="page-header">
      <h1>Danh mục</h1>
    </div>
    <p class="muted">Danh mục là nhãn nằm trong một môn (ví dụ Toán: Đại số, Hình học, Ôn thi vào 10). Chọn môn để xem và thêm.</p>

    <section class="panel">
      <div class="toolbar">
        <hui-select
          label="Môn học"
          placeholder="Chọn môn"
          searchable
          :options="subjectOptions"
          :value="subjectId"
          @change="selectSubject($event.detail.value)"
        />
        <form v-if="subjectId" class="toolbar grow" novalidate @submit.prevent="create">
          <hui-input v-model="newName" class="grow" label="Thêm danh mục" placeholder="Ví dụ: Đại số" :error="fields.name" />
          <hui-button type="submit" :loading="loading">Thêm</hui-button>
        </form>
      </div>
      <hui-alert v-if="error" severity="error" :description="error" />
    </section>

    <section v-if="subjectId" class="panel">
      <hui-table
        row-key="id"
        pagination="none"
        hover
        :columns="columns"
        :data="tags"
        :loading="loadingList"
        empty-text="Môn này chưa có danh mục dùng chung"
        @hui-action="onAction"
      />
    </section>

    <ItemFormModal v-model:open="editOpen" title="Sửa danh mục" :item="editing" :saveFn="saveEdit" @saved="onSaved" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref } from 'vue';

import { adminApi, catalogApi } from '../../api';
import ItemFormModal from '../../components/ItemFormModal.vue';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';

const { data: master, load } = useMasterData();
const subjectId = ref('');
const tags = ref([]);
const loadingList = ref(false);
const newName = ref('');
const editOpen = ref(false);
const editing = ref(null);
const { loading, error, fields, run } = useSubmit();

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ label: s.name, value: s.id })));

const columns = [
  { key: 'name', title: 'Tên danh mục' },
  { key: 'slug', title: 'Slug' },
  { key: 'sortOrder', title: 'Thứ tự', type: 'number', width: '90px' },
  { key: 'isActive', title: 'Trạng thái', type: 'tag', labels: { true: 'Đang dùng', false: 'Đã ẩn' }, colors: { true: 'success', false: 'secondary' } },
  { key: 'actions', title: 'Thao tác', type: 'actions', actions: [{ key: 'edit', label: 'Sửa' }] },
];

async function refresh() {
  if (!subjectId.value) return;
  loadingList.value = true;
  try {
    tags.value = await adminApi.tags(subjectId.value);
  } finally {
    loadingList.value = false;
  }
}

function selectSubject(id) {
  subjectId.value = id || '';
  tags.value = [];
  refresh();
}

async function create() {
  const res = await run(() => catalogApi.createTag(subjectId.value, { name: newName.value.trim() }));
  if (res.ok) {
    newName.value = '';
    message.success(`Đã thêm danh mục ${res.data.name}.`);
    refresh();
  }
}

function onAction({ detail: { action, row } }) {
  if (action === 'edit') {
    editing.value = row;
    editOpen.value = true;
  }
}

const saveEdit = (changes) => adminApi.updateTag(editing.value.id, changes);

function onSaved() {
  message.success('Đã lưu.');
  refresh();
}

onMounted(load);
</script>

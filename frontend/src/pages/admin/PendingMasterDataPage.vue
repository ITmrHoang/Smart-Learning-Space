<template>
  <div class="page">
    <div class="page-header">
      <h1>Mục giáo viên thêm</h1>
      <hui-button variant="outline" :loading="loadingList" @hui-click="refresh">Tải lại</hui-button>
    </div>
    <p class="muted">
      Lớp riêng và danh mục do giáo viên tự thêm, hiện chỉ người thêm nhìn thấy. Đưa lên dùng chung để mọi người dùng được,
      hoặc gộp vào một mục dùng chung đã có nếu bị trùng.
    </p>

    <section class="panel">
      <hui-alert v-if="listError" severity="error" :description="listError" />
      <hui-table
        row-key="id"
        pagination="none"
        hover
        :columns="columns"
        :data="rows"
        :loading="loadingList"
        empty-text="Không có mục nào chờ xử lý"
        @hui-action="onAction"
      />
    </section>

    <ConfirmModal
      v-model:open="promoteOpen"
      title="Đưa lên dùng chung"
      :message="selected ? `Đưa &quot;${selected.name}&quot; lên dùng chung cho mọi người?` : ''"
      ok-text="Đưa lên"
      :loading="working"
      @confirm="promote"
    />

    <hui-modal
      :open="mergeOpen"
      :title="selected ? `Gộp &quot;${selected.name}&quot; vào…` : 'Gộp'"
      ok-text="Gộp"
      cancel-text="Hủy"
      :confirmLoading="working"
      @hui-ok="onMergeOk"
      @hui-close="mergeOpen = false"
    >
      <div class="form-grid">
        <hui-alert v-if="mergeError" severity="error" :description="mergeError" />
        <hui-select
          label="Mục dùng chung"
          placeholder="Chọn mục"
          searchable
          :options="mergeOptions"
          :value="mergeInto"
          @change="mergeInto = $event.detail.value"
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
import { useMasterData } from '../../utils/masterData';

const items = ref([]);
const loadingList = ref(false);
const listError = ref('');
const selected = ref(null);
const promoteOpen = ref(false);
const mergeOpen = ref(false);
const mergeOptions = ref([]);
const mergeInto = ref('');
const mergeError = ref('');
const working = ref(false);
const { load: reloadMasterData } = useMasterData();

const columns = [
  { key: 'kind', title: 'Loại', type: 'tag', labels: { grade: 'Lớp', tag: 'Danh mục' }, colors: { grade: 'primary', tag: 'info' } },
  { key: 'name', title: 'Tên' },
  { key: 'subjectName', title: 'Môn' },
  { key: 'ownerUsername', title: 'Giáo viên' },
  { key: 'createdAt', title: 'Ngày thêm', type: 'datetime' },
  {
    key: 'actions',
    title: 'Thao tác',
    type: 'actions',
    actions: [
      { key: 'promote', label: 'Đưa lên dùng chung' },
      { key: 'merge', label: 'Gộp vào…' },
    ],
  },
];

const rows = computed(() => items.value.map((i) => ({ ...i, subjectName: i.subjectName ?? '—' })));

async function refresh() {
  loadingList.value = true;
  listError.value = '';
  try {
    items.value = await adminApi.pendingMasterData();
  } catch (e) {
    listError.value = e.message;
  } finally {
    loadingList.value = false;
  }
}

async function onAction({ detail: { action, row } }) {
  selected.value = row;
  if (action === 'promote') {
    promoteOpen.value = true;
    return;
  }
  mergeError.value = '';
  mergeInto.value = '';
  const targets = row.kind === 'grade' ? await adminApi.grades() : await adminApi.tags(row.subjectId);
  mergeOptions.value = targets.filter((t) => t.isActive && t.id !== row.id).map((t) => ({ label: t.name, value: t.id }));
  mergeOpen.value = true;
}

async function promote() {
  working.value = true;
  try {
    const row = selected.value;
    await (row.kind === 'grade' ? adminApi.promoteGrade(row.id) : adminApi.promoteTag(row.id));
    message.success(`Đã đưa "${row.name}" lên dùng chung.`);
    promoteOpen.value = false;
    refresh();
    reloadMasterData(true);
  } catch (e) {
    message.error(e.message);
  } finally {
    working.value = false;
  }
}

async function onMergeOk(e) {
  e.preventDefault();
  if (!mergeInto.value) {
    mergeError.value = 'Hãy chọn mục để gộp vào.';
    return;
  }
  working.value = true;
  try {
    const row = selected.value;
    await (row.kind === 'grade' ? adminApi.mergeGrade(row.id, mergeInto.value) : adminApi.mergeTag(row.id, mergeInto.value));
    message.success(`Đã gộp "${row.name}".`);
    mergeOpen.value = false;
    refresh();
  } catch (err) {
    mergeError.value = err.fields?.intoId?.[0] ?? err.message;
  } finally {
    working.value = false;
  }
}

onMounted(refresh);
</script>

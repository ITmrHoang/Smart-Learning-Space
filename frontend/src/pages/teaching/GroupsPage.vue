<template>
  <div class="page">
    <div class="page-header">
      <h1>Nhóm học sinh</h1>
      <hui-button @hui-click="edit(null)">Tạo nhóm</hui-button>
    </div>
    <p class="muted small">
      Gom học sinh từ các lớp học của bạn thành nhóm (nhóm ôn thi, nhóm kèm riêng…) để mời vào buổi học một lần cho cả nhóm.
    </p>

    <hui-alert v-if="error" severity="error" :description="error" />
    <div v-if="loaded && !groups.length" class="panel empty muted">Chưa có nhóm nào. Bấm "Tạo nhóm" để bắt đầu.</div>
    <div class="card-list">
      <article v-for="g in groups" :key="g.id" class="card">
        <h3>{{ g.name }}</h3>
        <p class="muted small" style="margin: 0">
          {{ g.memberCount }} học sinh<template v-if="g.classTitle"> · lớp học {{ g.classTitle }}</template>
        </p>
        <p class="muted small" style="margin: 0">Cập nhật {{ formatDateTime(g.updatedAt) }}</p>
        <div class="row">
          <hui-button size="sm" variant="outline" @hui-click="edit(g.id)">Sửa</hui-button>
          <hui-button size="sm" variant="text" @hui-click="remove(g)">Xóa</hui-button>
        </div>
      </article>
    </div>

    <GroupFormModal v-model:open="formOpen" :groupId="editing" @saved="onSaved" />
    <ConfirmModal
      v-model:open="confirmOpen"
      title="Xóa nhóm?"
      :message="`Xóa nhóm “${removing?.name}”. Buổi học đã mời nhóm này sẽ không còn mời các thành viên qua nhóm nữa.`"
      ok-text="Xóa"
      danger
      @confirm="doRemove"
    />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import ConfirmModal from '../../components/ConfirmModal.vue';
import GroupFormModal from '../../components/teaching/GroupFormModal.vue';
import { formatDateTime } from '../../utils/form';

const groups = ref([]);
const loaded = ref(false);
const error = ref('');
const formOpen = ref(false);
const editing = ref(null);
const confirmOpen = ref(false);
const removing = ref(null);

async function load() {
  error.value = '';
  try {
    groups.value = await teachingApi.groups();
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
}

function edit(id) {
  editing.value = id;
  formOpen.value = true;
}

function onSaved() {
  message.success(editing.value ? 'Đã lưu nhóm.' : 'Đã tạo nhóm.');
  load();
}

function remove(g) {
  removing.value = g;
  confirmOpen.value = true;
}

async function doRemove() {
  try {
    await teachingApi.deleteGroup(removing.value.id);
    message.success('Đã xóa nhóm.');
    load();
  } catch (e) {
    message.error(e.message);
  } finally {
    confirmOpen.value = false;
  }
}

onMounted(load);
</script>

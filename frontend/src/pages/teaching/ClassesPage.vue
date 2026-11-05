<template>
  <div class="page">
    <div class="page-header">
      <h1>Lớp học của tôi (giáo viên)</h1>
      <div class="row">
        <hui-segmented :options="tabOptions" :value="archived ? 'archived' : 'active'" @change="setArchived($event.detail.value === 'archived')" />
        <hui-button @hui-click="formOpen = true">Tạo lớp học</hui-button>
      </div>
    </div>
    <p class="muted small">Lớp học kiểu "tự học" là khóa học học sinh học lúc nào cũng được. Nội dung của lớp học là lộ trình: bài học, đề ôn tập / kiểm tra.</p>

    <hui-alert v-if="list.error.value" severity="error" :description="list.error.value" />
    <div v-if="!list.loading.value && !list.items.value.length" class="panel empty muted">
      {{ archived ? 'Chưa có lớp học nào được lưu trữ.' : 'Chưa có lớp học nào. Bấm "Tạo lớp học" để bắt đầu.' }}
    </div>
    <div class="card-list">
      <article v-for="c in list.items.value" :key="c.id" class="card">
        <RouterLink class="title" :to="{ name: 'teach-class', params: { id: c.id } }"><h3>{{ c.title }}</h3></RouterLink>
        <div class="row">
          <hui-tag size="sm">{{ subjectName(c.subjectId) }}</hui-tag>
          <hui-tag size="sm" color="info">{{ MODE_LABELS[c.mode] }}</hui-tag>
          <hui-tag size="sm" :color="c.visibility === 'public' ? 'success' : 'secondary'">{{ VISIBILITY_LABELS[c.visibility] }}</hui-tag>
        </div>
        <p class="muted small">
          {{ c.studentCount }} học sinh<span v-if="c.pendingCount"> · <b>{{ c.pendingCount }} chờ duyệt</b></span> · {{ c.stepCount }} bước
        </p>
        <p class="small">
          <span v-if="c.pathVersion == null" class="field-error">Lộ trình chưa xuất bản</span>
          <span v-else-if="c.pathHasChanges">Lộ trình v{{ c.pathVersion }} · có thay đổi chưa xuất bản</span>
          <span v-else class="muted">Lộ trình v{{ c.pathVersion }}</span>
        </p>
        <RouterLink :to="{ name: 'teach-class', params: { id: c.id } }">Quản lý →</RouterLink>
      </article>
    </div>
    <hui-pagination class="pager" :total="list.total.value" :current="list.page.value" :pageSize="list.pageSize.value" hide-on-single-page @change="list.onPage($event.detail)" />

    <ClassFormModal v-model:open="formOpen" @saved="onCreated" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import ClassFormModal from '../../components/teaching/ClassFormModal.vue';
import { usePagedList } from '../../utils/list';
import { useMasterData } from '../../utils/masterData';
import { MODE_LABELS, VISIBILITY_LABELS } from '../../utils/teaching';

const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const list = usePagedList((q) => teachingApi.classes(q), { archived: false }, { pageSize: 24 });
const archived = ref(false);
const formOpen = ref(false);
const tabOptions = [
  { value: 'active', label: 'Đang dạy' },
  { value: 'archived', label: 'Đã lưu trữ' },
];
const subjectName = (id) => master.value?.subjects.find((s) => s.id === id)?.name ?? '';

function setArchived(v) {
  archived.value = v;
  list.setFilter('archived', v);
}

function onCreated(c) {
  message.success('Đã tạo lớp học. Hãy thêm bài / đề vào lộ trình rồi xuất bản.');
  router.push({ name: 'teach-class', params: { id: c.id } });
}

onMounted(() => {
  loadMaster();
  list.load();
});
</script>

<style scoped>
.pager {
  display: flex;
  justify-content: flex-end;
}
</style>

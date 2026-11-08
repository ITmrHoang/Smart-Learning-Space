<template>
  <div class="page">
    <div class="page-header">
      <h1>Lịch dạy</h1>
      <div class="row">
        <hui-select class="class-filter" :options="classOptions" :value="classId" placeholder="Tất cả lớp học" clearable label="Lọc theo lớp học" @change="setClass($event.detail.value)" />
        <hui-button @hui-click="create()">Tạo buổi học</hui-button>
      </div>
    </div>
    <p class="muted small">
      Buổi học kiểu Google Meet: đặt lịch một lần hoặc lặp hằng tuần, mời lớp học / nhóm / từng học sinh, gắn link phòng học.
      Học sinh nhận thông báo, email kèm lịch (.ics) và được nhắc trước giờ học.
    </p>

    <ScheduleCalendar :key="classId ?? 'all'" ref="calendar" :fetcher="fetch" selectable @select="create" @open="open" />

    <SessionFormModal v-model:open="formOpen" :preset="preset" @saved="calendar?.reload()" />
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import ScheduleCalendar from '../../components/ScheduleCalendar.vue';
import SessionFormModal from '../../components/teaching/SessionFormModal.vue';

const route = useRoute();
const router = useRouter();
const calendar = ref(null);
const classes = ref([]);
const classId = ref(route.query.classId || null);
const formOpen = ref(false);
const preset = ref(null);
const classOptions = computed(() => classes.value.map((c) => ({ value: c.id, label: c.title })));

const fetch = (range) => teachingApi.sessions({ ...range, classId: classId.value || undefined });

function setClass(id) {
  classId.value = id || null;
  router.replace({ query: id ? { classId: id } : {} });
}

/** Tạo buổi: từ nút (giờ tròn kế tiếp) hoặc từ khung giờ kéo chọn trên lịch. */
function create(slot = null) {
  preset.value = { start: slot?.start ?? null, end: slot?.end ?? null, classId: classId.value };
  formOpen.value = true;
}

function open(id) {
  router.push({ name: 'session', params: { id } });
}

onMounted(async () => {
  classes.value = (await teachingApi.classes({ pageSize: 100 }).catch(() => ({ items: [] }))).items;
});
</script>

<style scoped>
.class-filter {
  min-width: 220px;
}
</style>

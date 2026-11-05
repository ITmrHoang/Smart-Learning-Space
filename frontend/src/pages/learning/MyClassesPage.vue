<template>
  <div class="page">
    <div class="page-header">
      <h1>Lớp học của tôi</h1>
      <RouterLink to="/kham-pha">Tìm thêm lớp học →</RouterLink>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />
    <p v-if="!loading && !items.length" class="panel empty muted">
      Bạn chưa vào lớp học nào. <RouterLink to="/kham-pha">Tìm lớp học</RouterLink> hoặc nhập mã mời của giáo viên.
    </p>
    <div class="card-list">
      <ClassCard v-for="m in items" :key="m.class.id" :cls="m.class" :progress="m.class.myStatus === 'active' ? m.progress : null">
        <RouterLink v-if="m.class.myStatus === 'active'" :to="{ name: 'class', params: { id: m.class.id } }">
          {{ m.progress.done ? 'Học tiếp →' : 'Bắt đầu học →' }}
        </RouterLink>
        <span v-else class="muted small">Đang chờ giáo viên duyệt</span>
      </ClassCard>
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';

import { learnApi } from '../../api';
import ClassCard from '../../components/learning/ClassCard.vue';
import { useMasterData } from '../../utils/masterData';

const { load: loadMaster } = useMasterData();
const items = ref([]);
const loading = ref(true);
const error = ref('');

onMounted(async () => {
  loadMaster();
  try {
    items.value = await learnApi.myClasses();
  } catch (e) {
    error.value = e.message;
  } finally {
    loading.value = false;
  }
});
</script>

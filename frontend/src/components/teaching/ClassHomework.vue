<template>
  <div class="stack">
    <div class="row between">
      <h2 style="margin: 0">Bài tập về nhà</h2>
      <hui-button size="sm" variant="text" @hui-click="load">Tải lại</hui-button>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />
    <p v-if="loaded && !items.length" class="muted small">
      Lộ trình đã xuất bản chưa có bài tập. Thêm bằng nút "+ Bài tập về nhà" ở tab Lộ trình
      (soạn bài tập ở <RouterLink :to="{ name: 'teach-assignments' }">Bài tập về nhà</RouterLink>).
    </p>
    <div class="table-wrap">
      <table v-if="items.length" class="data">
        <thead>
          <tr>
            <th>Bài tập</th>
            <th>Hạn nộp</th>
            <th>Đã nộp</th>
            <th>Chờ chấm</th>
            <th>Đã chấm</th>
            <th>Cần làm lại</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="h in items" :key="h.stepId">
            <td>
              <b>{{ h.title }}</b>
              <div class="muted small">{{ h.maxScore != null ? `Thang ${formatScore(h.maxScore)}` : 'Chỉ nhận xét' }}</div>
            </td>
            <td>
              <template v-if="h.dueAt">
                {{ formatDateTime(h.dueAt) }}
                <hui-tag v-if="new Date(h.dueAt) < now" size="sm" color="secondary">Đã hết hạn</hui-tag>
              </template>
              <span v-else class="muted">—</span>
            </td>
            <td>
              {{ h.submitted + h.graded + h.returned }}/{{ h.assigned }}
              <span v-if="h.late" class="muted small">({{ h.late }} muộn)</span>
            </td>
            <td><hui-tag v-if="h.submitted" size="sm" color="info">{{ h.submitted }}</hui-tag><span v-else>0</span></td>
            <td>{{ h.graded }}</td>
            <td>{{ h.returned }}</td>
            <td>
              <RouterLink :to="{ name: 'teach-submissions', params: { id: classId, stepId: h.stepId } }">
                {{ h.submitted ? 'Chấm bài →' : 'Xem bài nộp →' }}
              </RouterLink>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { formatScore } from '../../utils/teaching';

const props = defineProps({ classId: { type: String, required: true } });
const items = ref([]);
const loaded = ref(false);
const error = ref('');
const now = new Date();

async function load() {
  error.value = '';
  try {
    items.value = await teachingApi.classHomework(props.classId);
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
}

onMounted(load);
</script>

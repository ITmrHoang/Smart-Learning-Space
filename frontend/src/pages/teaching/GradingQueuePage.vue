<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <h1>Bài chờ chấm</h1>
        <span class="muted small">Bài kiểm tra đã nộp còn câu tự luận chưa chấm — đề của bạn và lớp bạn làm trợ giảng. Bài nộp trước ở trên.</span>
      </div>
      <hui-button variant="outline" :loading="loading" @hui-click="load">Tải lại</hui-button>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />
    <p v-if="!loading && !error && !items.length" class="panel muted">Không còn bài nào chờ chấm. 🎉</p>

    <section v-for="g in groups" :key="g.examId" class="panel stack">
      <div class="row between">
        <b>{{ g.examTitle }}</b>
        <span class="muted small">{{ g.items.length }} bài · {{ g.items.reduce((s, i) => s + i.pending, 0) }} câu</span>
      </div>
      <div class="table-wrap">
      <table class="data">
        <thead>
          <tr>
            <th>Học sinh</th>
            <th>Lớp học</th>
            <th>Nộp lúc</th>
            <th>Điểm tạm</th>
            <th>Chưa chấm</th>
            <th />
          </tr>
        </thead>
        <tbody>
          <tr v-for="it in g.items" :key="it.attemptId">
            <td>{{ it.student.fullName }} <span class="muted small">@{{ it.student.username }}</span></td>
            <td>{{ it.classTitle ?? '—' }}</td>
            <td class="small">{{ formatDateTime(it.submittedAt) }}</td>
            <td>{{ formatScore(it.score) }} / {{ formatScore(it.maxScore) }}</td>
            <td>{{ it.pending }} câu</td>
            <td>
              <RouterLink :to="{ name: 'teach-grade-attempt', params: { id: it.attemptId } }">
                <hui-button size="sm">Chấm</hui-button>
              </RouterLink>
            </td>
          </tr>
        </tbody>
      </table>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { formatScore } from '../../utils/teaching';

const items = ref([]);
const loading = ref(false);
const error = ref('');

/** Gom theo đề, giữ thứ tự bài nộp trước. */
const groups = computed(() => {
  const map = new Map();
  for (const it of items.value) {
    if (!map.has(it.examId)) map.set(it.examId, { examId: it.examId, examTitle: it.examTitle, items: [] });
    map.get(it.examId).items.push(it);
  }
  return [...map.values()];
});

async function load() {
  loading.value = true;
  error.value = '';
  try {
    items.value = await teachingApi.gradingQueue();
  } catch (e) {
    error.value = e.message;
  } finally {
    loading.value = false;
  }
}

onMounted(load);
</script>

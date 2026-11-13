<template>
  <div class="page">
    <RouterLink :to="{ name: 'teach-class', params: { id: classId }, query: { tab: 'bai-tap' } }" class="small">← Bài tập của lớp học</RouterLink>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="data">
      <div class="page-header">
        <div class="stack" style="gap: 4px">
          <h1>{{ data.step.title }}</h1>
          <span class="muted small">
            <template v-if="data.step.dueAt">Hạn nộp {{ formatDateTime(data.step.dueAt) }} · </template>
            {{ data.step.maxScore != null ? `Thang điểm ${formatScore(data.step.maxScore)}` : 'Chỉ nhận xét' }} ·
            {{ data.step.submitted + data.step.graded + data.step.returned }}/{{ data.step.assigned }} đã nộp
          </span>
        </div>
        <hui-button v-if="nextToGrade" @hui-click="open(nextToGrade)">Chấm bài tiếp theo</hui-button>
      </div>
      <details class="panel">
        <summary><b>Đề bài</b></summary>
        <hui-markdown :value="data.instructions || '_(Không có đề bài)_'" />
      </details>
      <div class="panel table-wrap">
        <table class="data">
          <thead>
            <tr>
              <th>Học sinh</th>
              <th>Trạng thái</th>
              <th>Nộp lúc</th>
              <th>File</th>
              <th>Điểm</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in data.students" :key="s.userId">
              <td>{{ s.fullName }}<br /><span class="muted small">@{{ s.username }}</span></td>
              <td>
                <hui-tag v-if="s.submission" size="sm" :color="HOMEWORK_STATUS_COLORS[s.submission.status]">
                  {{ HOMEWORK_STATUS_LABELS[s.submission.status] }}
                </hui-tag>
                <hui-tag v-else size="sm" color="secondary">Chưa nộp</hui-tag>
                <hui-tag v-if="s.submission?.late" size="sm" color="danger" class="late">Muộn</hui-tag>
              </td>
              <td>{{ s.submission ? formatDateTime(s.submission.submittedAt) : '—' }}</td>
              <td>{{ s.submission?.fileCount ?? '—' }}</td>
              <td>{{ s.submission?.score != null ? formatScore(s.submission.score) : '—' }}</td>
              <td>
                <a v-if="s.submission" href="#" @click.prevent="open(s.submission.id)">
                  {{ s.submission.status === 'submitted' ? 'Chấm' : 'Xem' }}
                </a>
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </div>
</template>

<script setup>
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { HOMEWORK_STATUS_COLORS, HOMEWORK_STATUS_LABELS, formatScore } from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const classId = route.params.id;
const data = ref(null);
const error = ref('');

const nextToGrade = computed(() => data.value?.students.find((s) => s.submission?.status === 'submitted')?.submission.id);

function open(id) {
  router.push({ name: 'teach-submission', params: { id }, query: { classId, stepId: route.params.stepId } });
}

onMounted(async () => {
  try {
    data.value = await teachingApi.stepSubmissions(classId, route.params.stepId);
  } catch (e) {
    error.value = e.message;
  }
});
</script>

<style scoped>
.late {
  margin-left: 4px;
}
</style>

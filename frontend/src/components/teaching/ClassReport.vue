<template>
  <div class="stack">
    <div class="row between">
      <h2 style="margin: 0">Báo cáo lớp học</h2>
      <span class="row">
        <hui-button size="sm" variant="text" :loading="loading" @hui-click="load">Tải lại</hui-button>
        <hui-button size="sm" variant="outline" :loading="downloading" @hui-click="download">Tải bảng điểm Excel</hui-button>
      </span>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />

    <template v-if="r">
      <div class="kpis">
        <div class="kpi"><b>{{ r.students.length }}</b><span>học sinh</span></div>
        <div class="kpi"><b>{{ avgProgress }}%</b><span>tiến độ trung bình</span></div>
        <div class="kpi"><b>{{ avgScore == null ? '—' : `${formatScore(avgScore)}%` }}</b><span>điểm trung bình</span></div>
        <div class="kpi" :class="{ warn: attention.length }"><b>{{ attention.length }}</b><span>cần chú ý</span></div>
      </div>

      <div v-if="attention.length" class="panel-lite stack">
        <h3 style="margin: 0">Học sinh cần chú ý</h3>
        <ul class="clean-list">
          <li v-for="s in attention" :key="s.userId" class="row between">
            <span>{{ s.fullName }} <span class="muted small">@{{ s.username }}</span></span>
            <span class="row">
              <hui-tag v-for="f in s.flags" :key="f" size="sm" :color="FLAG_COLORS[f]">{{ FLAG_LABELS[f] }}</hui-tag>
              <hui-button v-if="s.guardians" size="sm" variant="text" @hui-click="openSend(s)">Gửi phụ huynh</hui-button>
            </span>
          </li>
        </ul>
        <p class="muted small" style="margin: 0">
          Lâu không học: không có hoạt động {{ 7 }} ngày · Tiến độ thấp: dưới 30% sau 7 ngày vào lớp · Điểm thấp: điểm trung bình các đề dưới 50%.
        </p>
      </div>

      <div class="stack">
        <h3 style="margin: 0">Bảng điểm (điểm cao nhất mỗi đề)</h3>
        <p v-if="!r.students.length" class="muted small">Chưa có học sinh.</p>
        <div v-else class="table-wrap">
          <table class="grid">
            <thead>
              <tr>
                <th>Học sinh</th>
                <th>Tiến độ</th>
                <th v-for="e in r.exams" :key="e.stepId" :title="e.title">
                  {{ e.title }}<br /><span class="muted small">{{ e.submitted ? `TB ${formatScore(e.avgScore)}/${formatScore(e.maxScore)} · ${e.submitted} bài` : 'chưa có bài nộp' }}</span>
                </th>
                <th>Điểm TB</th>
                <th>Buổi học</th>
                <th>Hoạt động gần nhất</th>
                <th>Phụ huynh</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="s in r.students" :key="s.userId">
                <td>{{ s.fullName }}<br /><span class="muted small">@{{ s.username }}</span></td>
                <td>{{ s.progress.percent }}%</td>
                <td v-for="(score, i) in s.scores" :key="i" :class="scoreClass(score, r.exams[i].maxScore)">
                  {{ score == null ? '—' : formatScore(score) }}
                </td>
                <td>{{ s.avgPercent == null ? '—' : `${formatScore(s.avgPercent)}%` }}</td>
                <td>{{ s.sessionsTotal ? `${s.sessionsAttended}/${s.sessionsTotal}` : '—' }}</td>
                <td class="small">{{ s.lastActiveAt ? formatDateTime(s.lastActiveAt) : 'Chưa học' }}</td>
                <td>
                  <hui-button v-if="s.guardians" size="sm" variant="text" @hui-click="openSend(s)">Gửi báo cáo</hui-button>
                  <span v-else class="muted small" title="Học sinh khai email phụ huynh ở trang Hồ sơ, phụ huynh bấm xác nhận">chưa có</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <div class="grid-2">
        <div class="stack">
          <h3 style="margin: 0">Theo danh mục</h3>
          <p v-if="!r.topics.length" class="muted small">Chưa có câu hỏi gắn danh mục trong các bài đã nộp.</p>
          <div v-for="t in r.topics" :key="t.id" class="bar-row">
            <span class="bar-label">{{ t.name }} <span class="muted small">({{ t.answers }} lượt)</span></span>
            <div class="bar"><span :class="barClass(t.percent)" :style="{ width: `${t.percent}%` }" /></div>
            <b class="bar-value">{{ formatScore(t.percent) }}%</b>
          </div>
        </div>
        <div class="stack">
          <h3 style="margin: 0">Theo mức độ</h3>
          <p v-if="!r.levels.length" class="muted small">Chưa có bài nộp.</p>
          <div v-for="l in r.levels" :key="l.name" class="bar-row">
            <span class="bar-label">{{ LEVEL_LABELS[l.name] ?? l.name }} <span class="muted small">({{ l.answers }} lượt)</span></span>
            <div class="bar"><span :class="barClass(l.percent)" :style="{ width: `${l.percent}%` }" /></div>
            <b class="bar-value">{{ formatScore(l.percent) }}%</b>
          </div>
        </div>
      </div>

      <div class="stack">
        <h3 style="margin: 0">Câu sai nhiều nhất</h3>
        <p v-if="!r.hardQuestions.length" class="muted small">Chưa có câu nào bị làm sai.</p>
        <ol class="hard">
          <li v-for="q in r.hardQuestions" :key="q.questionId">
            <hui-markdown :value="q.content" />
            <span class="row small">
              <hui-tag size="sm" :color="LEVEL_COLORS[q.level]">{{ LEVEL_LABELS[q.level] }}</hui-tag>
              <span class="muted">{{ q.wrong }}/{{ q.answers }} lượt sai · tỷ lệ điểm {{ formatScore(q.percent) }}%</span>
            </span>
          </li>
        </ol>
      </div>
      <p class="muted small" style="margin: 0">Số liệu tính trên các bài đã nộp trong lớp học (không tính làm thử) · cập nhật {{ formatDateTime(r.generatedAt) }}</p>
    </template>

    <hui-modal :open="sendOpen" :title="`Gửi báo cáo cho phụ huynh của ${sending?.fullName ?? ''}`" ok-text="Gửi" cancel-text="Hủy" :confirmLoading="sendLoading" @hui-ok="send" @hui-close="sendOpen = false">
      <div class="form-grid">
        <p class="small" style="margin: 0">Email gồm tiến độ, điểm, buổi học 7 ngày gần nhất của lớp học này, gửi tới {{ sending?.guardians }} phụ huynh đã xác nhận.</p>
        <hui-textarea label="Nhận xét của giáo viên (không bắt buộc)" :rows="4" :value="note" placeholder="Ví dụ: Em tiến bộ ở phần hệ phương trình, cần luyện thêm bài toán thực tế." @input="note = $event.detail.value" />
      </div>
    </hui-modal>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { LEVEL_COLORS, LEVEL_LABELS, formatScore } from '../../utils/teaching';

const FLAG_LABELS = { inactive: 'Lâu không học', low_progress: 'Tiến độ thấp', low_score: 'Điểm thấp' };
const FLAG_COLORS = { inactive: 'secondary', low_progress: 'warning', low_score: 'danger' };

const props = defineProps({ classId: { type: String, required: true } });
const r = ref(null);
const loading = ref(false);
const downloading = ref(false);
const error = ref('');
const sendOpen = ref(false);
const sending = ref(null);
const note = ref('');
const sendLoading = ref(false);

const attention = computed(() => (r.value?.students ?? []).filter((s) => s.flags.length));
const avgProgress = computed(() => {
  const list = r.value?.students ?? [];
  return list.length ? Math.round(list.reduce((a, s) => a + s.progress.percent, 0) / list.length) : 0;
});
const avgScore = computed(() => {
  const list = (r.value?.students ?? []).filter((s) => s.avgPercent != null);
  return list.length ? list.reduce((a, s) => a + s.avgPercent, 0) / list.length : null;
});

const barClass = (p) => (p < 50 ? 'low' : p < 75 ? 'mid' : 'high');
function scoreClass(score, max) {
  if (score == null || !max) return '';
  return score / max < 0.5 ? 'low' : score / max >= 0.8 ? 'high' : '';
}

async function load() {
  loading.value = true;
  error.value = '';
  try {
    r.value = await teachingApi.classReport(props.classId);
  } catch (e) {
    error.value = e.message;
  } finally {
    loading.value = false;
  }
}

async function download() {
  downloading.value = true;
  try {
    const blob = await teachingApi.classReportXlsx(props.classId);
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `bang-diem-${new Date().toISOString().slice(0, 10)}.xlsx`;
    a.click();
    URL.revokeObjectURL(url);
  } catch (e) {
    message.error(e.message);
  } finally {
    downloading.value = false;
  }
}

function openSend(s) {
  sending.value = s;
  note.value = '';
  sendOpen.value = true;
}

async function send(e) {
  e.preventDefault();
  sendLoading.value = true;
  try {
    const res = await teachingApi.sendGuardianReport(props.classId, sending.value.userId, note.value.trim());
    message.success(`Đã gửi báo cáo cho ${res.sent} phụ huynh.`);
    sendOpen.value = false;
  } catch (err) {
    message.error(err.message);
  } finally {
    sendLoading.value = false;
  }
}

onMounted(load);
</script>

<style scoped>
.grid-2 > .stack {
  align-content: start;
}
.kpis {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 12px;
}
.kpi {
  display: grid;
  gap: 2px;
  padding: 12px 14px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.kpi b {
  font-size: 1.5rem;
}
.kpi span {
  color: var(--hui-text-secondary, #6b7280);
  font-size: 0.875rem;
}
.kpi.warn b {
  color: var(--hui-danger, #be1b1b);
}
.panel-lite {
  padding: 12px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-fill-tertiary, #f5f8ff);
}
.table-wrap {
  overflow-x: auto;
}
.grid {
  border-collapse: collapse;
  min-width: 100%;
  font-size: 0.9rem;
}
.grid th,
.grid td {
  padding: 8px 10px;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
  text-align: left;
  white-space: nowrap;
  vertical-align: top;
}
.grid th {
  font-weight: 600;
  font-size: 0.8125rem;
}
.grid td.low {
  color: var(--hui-danger, #be1b1b);
  font-weight: 600;
}
.grid td.high {
  color: var(--hui-success, #10b981);
  font-weight: 600;
}
.bar-row {
  display: grid;
  grid-template-columns: minmax(0, 1.4fr) minmax(80px, 1fr) 3.5rem;
  gap: 10px;
  align-items: center;
}
.bar {
  height: 8px;
  border-radius: 999px;
  background: var(--hui-border-secondary, #eceef1);
  overflow: hidden;
}
.bar span {
  display: block;
  height: 100%;
}
.bar .low {
  background: var(--hui-danger, #be1b1b);
}
.bar .mid {
  background: var(--hui-warning, #f59e0b);
}
.bar .high {
  background: var(--hui-success, #10b981);
}
.bar-value {
  text-align: right;
  font-variant-numeric: tabular-nums;
}
.hard {
  margin: 0;
  padding-left: 1.25rem;
  display: grid;
  gap: 10px;
}
.hard li {
  padding-bottom: 8px;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
</style>

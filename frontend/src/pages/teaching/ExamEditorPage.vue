<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <RouterLink to="/day-hoc/de" class="small">← Danh sách đề</RouterLink>
        <h1>{{ isNew ? 'Tạo đề' : form.title || 'Đề' }}</h1>
      </div>
      <div class="row">
        <span v-if="exam" class="muted small">{{ exam.questionCount }} câu · {{ formatScore(exam.maxScore) }} điểm · {{ exam.attemptCount }} lượt làm</span>
        <hui-button v-if="exam" variant="outline" :loading="previewing" @hui-click="preview">Làm thử</hui-button>
        <hui-button :loading="loading" @hui-click="save">Lưu đề</hui-button>
      </div>
    </div>

    <hui-alert v-if="error" severity="error" :description="error" />
    <hui-alert v-for="w in exam?.warnings ?? []" :key="w" severity="warning" :description="w" />

    <section class="panel form-grid">
      <h2>Thông tin</h2>
      <div class="grid-3">
        <hui-input v-model="form.title" label="Tên đề" :error="fields.title" required />
        <hui-select label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="onSubject($event.detail.value)" />
        <hui-select label="Lớp (không bắt buộc)" allow-clear :options="gradeOptions" :value="form.gradeId" @change="form.gradeId = $event.detail.value || null" />
      </div>
      <div class="grid-3">
        <hui-select label="Loại đề" :options="kindOptions" :value="form.kind" @change="form.kind = $event.detail.value" />
        <hui-input-number label="Thời gian làm bài (phút, bỏ trống = không giới hạn)" :min="1" :max="600" :value="form.timeLimitMinutes" :error="fields.timeLimitMinutes" @change="form.timeLimitMinutes = $event.detail.value || null" />
        <hui-input-number label="Số lần làm tối đa (bỏ trống = không giới hạn)" :min="1" :max="100" :value="form.maxAttempts" :error="fields.maxAttempts" @change="form.maxAttempts = $event.detail.value || null" />
      </div>
      <div class="grid-3">
        <label class="field">
          <span>Mở đề lúc (không bắt buộc)</span>
          <input class="native-input" type="datetime-local" :value="toLocal(form.opensAt)" @change="form.opensAt = fromLocal($event.target.value)" />
        </label>
        <label class="field">
          <span>Đóng đề lúc (không bắt buộc)</span>
          <input class="native-input" type="datetime-local" :value="toLocal(form.closesAt)" @change="form.closesAt = fromLocal($event.target.value)" />
          <span v-if="fields.closesAt" class="field-error">{{ fields.closesAt }}</span>
        </label>
        <hui-select label="Hiện đáp án cho học sinh" :options="showOptions" :value="form.showAnswers" :disabled="form.kind === 'practice'" @change="form.showAnswers = $event.detail.value" />
      </div>
      <div class="row">
        <hui-checkbox label="Trộn thứ tự câu trong từng phần" :checked="form.shuffleQuestions" @change="form.shuffleQuestions = $event.detail.checked" />
        <hui-checkbox label="Trộn thứ tự phương án" :checked="form.shuffleOptions" @change="form.shuffleOptions = $event.detail.checked" />
      </div>
      <hui-textarea label="Hướng dẫn / mô tả" :rows="2" :value="form.description" @input="form.description = $event.detail.value" />
    </section>

    <section class="panel stack">
      <div class="row between">
        <h2 style="margin: 0">Các phần của đề</h2>
        <div class="row">
          <hui-button size="sm" variant="outline" @hui-click="thptPreset">Dùng cấu trúc THPT (3 phần)</hui-button>
          <hui-button size="sm" @hui-click="addSection()">Thêm phần</hui-button>
        </div>
      </div>
      <p v-if="fields.sections" class="field-error">{{ fields.sections }}</p>

      <article v-for="(s, si) in form.sections" :key="s.key" class="section">
        <div class="grid-3">
          <hui-input v-model="s.title" label="Tên phần" />
          <hui-select label="Cách chấm" :options="scoringOptions" :value="s.scoring" @change="s.scoring = $event.detail.value" />
          <hui-input-number label="Điểm mỗi câu" :min="0" :max="100" :step="0.05" :precision="2" :value="s.pointsPerQuestion" @change="s.pointsPerQuestion = Number($event.detail.value) || 0" />
        </div>

        <div class="stack">
          <div class="row between">
            <b>Câu cố định ({{ s.questions.length }})</b>
            <hui-button size="sm" variant="outline" :disabled="!form.subjectId" @hui-click="openPicker(si)">Chọn câu từ ngân hàng</hui-button>
          </div>
          <ol v-if="s.questions.length" class="fixed">
            <li v-for="(q, qi) in s.questions" :key="q.id">
              <div class="row between">
                <span class="row">
                  <hui-tag size="sm">{{ QUESTION_KIND_LABELS[q.kind] }}</hui-tag>
                  <hui-tag size="sm" :color="LEVEL_COLORS[q.level]">{{ LEVEL_LABELS[q.level] }}</hui-tag>
                </span>
                <span class="row">
                  <button class="icon" :disabled="qi === 0" title="Lên" @click="move(s.questions, qi, -1)">↑</button>
                  <button class="icon" :disabled="qi === s.questions.length - 1" title="Xuống" @click="move(s.questions, qi, 1)">↓</button>
                  <button class="icon" title="Bỏ khỏi đề" @click="s.questions.splice(qi, 1)">✕</button>
                </span>
              </div>
              <hui-markdown :value="q.content" />
            </li>
          </ol>
        </div>

        <div class="stack">
          <div class="row between">
            <b>Ma trận — rút ngẫu nhiên cho mỗi lượt làm</b>
            <hui-button size="sm" variant="outline" @hui-click="addRule(s)">Thêm ô ma trận</hui-button>
          </div>
          <div v-for="(r, ri) in s.rules" :key="r.key" class="rule">
            <hui-select label="Danh mục" allow-clear placeholder="Mọi danh mục" :options="tagOptions" :value="r.tagId" @change="r.tagId = $event.detail.value || null" />
            <hui-select label="Mức độ" allow-clear placeholder="Mọi mức độ" :options="levelOptions" :value="r.level" @change="r.level = $event.detail.value || null" />
            <hui-select label="Dạng câu" allow-clear placeholder="Mọi dạng" :options="questionKindOptions" :value="r.kind" @change="r.kind = $event.detail.value || null" />
            <hui-select label="Thư mục" allow-clear placeholder="Mọi thư mục" :options="folderOptions" :value="r.folderId" @change="r.folderId = $event.detail.value || null" />
            <hui-input-number label="Số câu" :min="1" :max="100" :value="r.count" @change="r.count = Number($event.detail.value) || 1" />
            <span class="small" :class="r.available != null && r.available < r.count ? 'field-error' : 'muted'">
              {{ r.available != null ? `Có ${r.available} câu` : '' }}
            </span>
            <hui-button size="sm" variant="text" @hui-click="s.rules.splice(ri, 1)">Xóa</hui-button>
          </div>
        </div>

        <div class="row between">
          <span class="muted small">{{ s.questions.length + s.rules.reduce((n, r) => n + (r.count || 0), 0) }} câu / lượt</span>
          <hui-button size="sm" variant="text" :disabled="form.sections.length <= 1" @hui-click="form.sections.splice(si, 1)">Xóa phần này</hui-button>
        </div>
      </article>
    </section>

    <section v-if="exam" class="panel">
      <div class="row between">
        <h2 style="margin: 0">Lượt làm bài</h2>
        <hui-button size="sm" variant="text" @hui-click="loadAttempts">Tải lại</hui-button>
      </div>
      <p v-if="!attempts.length" class="muted small">Chưa có lượt làm nào.</p>
      <table v-else class="attempts">
        <thead>
          <tr><th>Học sinh</th><th>Bắt đầu</th><th>Trạng thái</th><th>Điểm</th><th /></tr>
        </thead>
        <tbody>
          <tr v-for="a in attempts" :key="a.id">
            <td>{{ a.student.fullName }} <span class="muted small">@{{ a.student.username }}</span> <hui-tag v-if="a.isPreview" size="sm">Làm thử</hui-tag></td>
            <td>{{ formatDateTime(a.startedAt) }}</td>
            <td>{{ a.status === 'submitted' ? 'Đã nộp' : 'Đang làm' }}</td>
            <td>{{ a.score == null ? '—' : `${formatScore(a.score)} / ${formatScore(a.maxScore)}` }}</td>
            <td><RouterLink :to="{ name: 'attempt', params: { id: a.id } }">Xem bài</RouterLink></td>
          </tr>
        </tbody>
      </table>
    </section>

    <QuestionPickerModal v-model:open="picker.open" :subjectId="form.subjectId" :exclude="allQuestionIds" @picked="onPicked" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { catalogApi, teachingApi } from '../../api';
import QuestionPickerModal from '../../components/teaching/QuestionPickerModal.vue';
import { flattenFolders } from '../../utils/folders';
import { formatDateTime, useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import {
  EXAM_KIND_LABELS,
  LEVEL_COLORS,
  LEVEL_LABELS,
  QUESTION_KIND_LABELS,
  SCORING_LABELS,
  SHOW_ANSWERS_LABELS,
  formatScore,
  toOptions,
} from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const { loading, error, fields, run } = useSubmit();
const isNew = computed(() => route.params.id === 'moi');
const exam = ref(null);
const attempts = ref([]);
const tags = ref([]);
const folders = ref([]);
const previewing = ref(false);
const picker = reactive({ open: false, section: 0 });
let seq = 0;

const form = reactive({
  subjectId: null,
  gradeId: null,
  title: '',
  description: '',
  kind: 'practice',
  timeLimitMinutes: null,
  maxAttempts: null,
  opensAt: null,
  closesAt: null,
  shuffleQuestions: true,
  shuffleOptions: true,
  showAnswers: 'after_submit',
  sections: [],
});

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ value: g.id, label: g.name })));
const tagOptions = computed(() => tags.value.map((t) => ({ value: t.id, label: t.name })));
const folderOptions = computed(() => flattenFolders(folders.value).map((f) => ({ value: f.id, label: f.path })));
const kindOptions = toOptions(EXAM_KIND_LABELS);
const showOptions = toOptions(SHOW_ANSWERS_LABELS);
const scoringOptions = toOptions(SCORING_LABELS);
const levelOptions = toOptions(LEVEL_LABELS);
const questionKindOptions = toOptions(QUESTION_KIND_LABELS);
const allQuestionIds = computed(() => form.sections.flatMap((s) => s.questions.map((q) => q.id)));

const section = (title = `Phần ${form.sections.length + 1}`, scoring = 'per_question', points = 1) => ({
  key: ++seq,
  title,
  scoring,
  pointsPerQuestion: points,
  questions: [],
  rules: [],
});

function addSection() {
  form.sections.push(section());
}

function thptPreset() {
  if (form.sections.some((s) => s.questions.length || s.rules.length) && !window.confirm('Thay các phần hiện tại bằng cấu trúc THPT?')) return;
  form.sections = [
    section('Phần I. Trắc nghiệm nhiều lựa chọn', 'per_question', 0.25),
    section('Phần II. Trắc nghiệm đúng / sai', 'thpt_true_false', 1),
    section('Phần III. Trả lời ngắn', 'per_question', 0.5),
  ];
}

function addRule(s) {
  s.rules.push({ key: ++seq, tagId: null, level: null, kind: null, folderId: null, count: 1, available: null });
}

function move(arr, i, d) {
  const [x] = arr.splice(i, 1);
  arr.splice(i + d, 0, x);
}

function openPicker(si) {
  picker.section = si;
  picker.open = true;
}

function onPicked(qs) {
  form.sections[picker.section].questions.push(...qs);
}

async function onSubject(id) {
  if (allQuestionIds.value.length && !window.confirm('Đổi môn sẽ bỏ các câu cố định đã chọn. Tiếp tục?')) return;
  form.subjectId = id;
  form.sections.forEach((s) => {
    s.questions = [];
    s.rules.forEach((r) => (r.tagId = null));
  });
  tags.value = id ? await catalogApi.tags(id).catch(() => []) : [];
}

const toLocal = (iso) => {
  if (!iso) return '';
  const d = new Date(iso);
  const pad = (n) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
const fromLocal = (v) => (v ? new Date(v).toISOString() : null);

function apply(e) {
  exam.value = e;
  Object.assign(form, {
    subjectId: e.subjectId,
    gradeId: e.gradeId,
    title: e.title,
    description: e.description,
    kind: e.kind,
    timeLimitMinutes: e.timeLimitMinutes,
    maxAttempts: e.maxAttempts,
    opensAt: e.opensAt,
    closesAt: e.closesAt,
    shuffleQuestions: e.shuffleQuestions,
    shuffleOptions: e.shuffleOptions,
    showAnswers: e.showAnswers,
    sections: e.sections.map((s) => ({
      key: ++seq,
      title: s.title,
      scoring: s.scoring,
      pointsPerQuestion: s.pointsPerQuestion,
      questions: s.questions,
      rules: s.rules.map((r) => ({ ...r, key: ++seq })),
    })),
  });
}

function payload() {
  return {
    subjectId: form.subjectId,
    gradeId: form.gradeId,
    title: form.title,
    description: form.description,
    kind: form.kind,
    timeLimitMinutes: form.timeLimitMinutes,
    maxAttempts: form.maxAttempts,
    opensAt: form.opensAt,
    closesAt: form.closesAt,
    shuffleQuestions: form.shuffleQuestions,
    shuffleOptions: form.shuffleOptions,
    showAnswers: form.kind === 'practice' ? 'after_submit' : form.showAnswers,
    sections: form.sections.map((s) => ({
      title: s.title,
      scoring: s.scoring,
      pointsPerQuestion: s.pointsPerQuestion,
      questionIds: s.questions.map((q) => q.id),
      rules: s.rules.map((r) => ({ tagId: r.tagId, level: r.level, kind: r.kind, folderId: r.folderId, count: r.count })),
    })),
  };
}

async function save() {
  const res = await run(() => (isNew.value ? teachingApi.createExam(payload()) : teachingApi.updateExam(exam.value.id, payload())));
  if (!res.ok) return false;
  apply(res.data);
  message.success('Đã lưu đề.');
  if (isNew.value) router.replace({ name: 'teach-exam', params: { id: res.data.id } });
  return true;
}

async function preview() {
  if (!(await save())) return;
  previewing.value = true;
  try {
    const attempt = await teachingApi.previewExam(exam.value.id);
    router.push({ name: 'attempt', params: { id: attempt.id } });
  } catch (e) {
    message.error(e.message);
  } finally {
    previewing.value = false;
  }
}

async function loadAttempts() {
  if (exam.value) attempts.value = await teachingApi.examAttempts(exam.value.id).catch(() => []);
}

onMounted(async () => {
  await loadMaster();
  folders.value = await teachingApi.folders().catch(() => []);
  if (isNew.value) {
    form.subjectId = master.value?.subjects[0]?.id ?? null;
    form.sections = [section('Phần I')];
  } else {
    try {
      apply(await teachingApi.exam(route.params.id));
      loadAttempts();
    } catch (e) {
      error.value = e.message;
      return;
    }
  }
  tags.value = form.subjectId ? await catalogApi.tags(form.subjectId).catch(() => []) : [];
});
</script>

<style scoped>
.field {
  display: grid;
  gap: 6px;
  font-size: 0.9rem;
}
.section {
  display: grid;
  gap: 14px;
  padding: 16px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-lg, 12px);
  background: var(--hui-bg-secondary, #f7f8fa);
}
.fixed {
  margin: 0;
  padding-left: 1.4em;
  display: grid;
  gap: 8px;
}
.fixed li {
  background: var(--hui-bg-container, #fff);
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
  padding: 8px 10px;
}
.rule {
  display: grid;
  grid-template-columns: repeat(4, minmax(120px, 1fr)) 100px auto auto;
  gap: 8px;
  align-items: end;
}
.icon {
  border: 0;
  background: none;
  cursor: pointer;
  font-size: 1rem;
  color: var(--hui-text-secondary, #6b7280);
}
.icon:disabled {
  opacity: 0.3;
  cursor: default;
}
.attempts {
  width: 100%;
  border-collapse: collapse;
  margin-top: 12px;
}
.attempts th,
.attempts td {
  text-align: left;
  padding: 8px;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
@media (max-width: 860px) {
  .rule {
    grid-template-columns: 1fr 1fr;
  }
}
</style>

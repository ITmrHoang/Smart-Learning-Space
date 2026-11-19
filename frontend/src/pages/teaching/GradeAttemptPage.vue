<template>
  <div class="page">
    <RouterLink :to="{ name: 'teach-grading' }" class="small">← Bài chờ chấm</RouterLink>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="view">
      <div class="page-header">
        <div class="stack" style="gap: 4px">
          <h1>{{ view.student?.fullName }} <span class="muted small">@{{ view.student?.username }}</span></h1>
          <span class="muted small">
            {{ view.title }} · nộp {{ formatDateTime(view.submittedAt) }} ·
            <RouterLink :to="{ name: 'attempt', params: { id: view.id } }">xem cả bài</RouterLink>
          </span>
        </div>
        <div class="score">
          <span class="muted small">Điểm{{ view.pending ? ` (còn ${view.pending} câu chưa chấm)` : '' }}</span>
          <b>{{ formatScore(view.score) }} / {{ formatScore(view.maxScore) }}</b>
        </div>
      </div>

      <hui-alert v-if="!essays.length" severity="info" description="Bài này không có câu tự luận." />
      <nav v-else class="row tabs" aria-label="Các câu tự luận">
        <button
          v-for="it in essays"
          :key="it.position"
          type="button"
          class="tab"
          :class="{ active: it.position === current?.position, done: !it.grading?.pending }"
          @click="select(it)"
        >
          Câu {{ numberOf(it) }} {{ it.grading?.pending ? '' : `· ${formatScore(it.grading.score)} đ` }}
        </button>
      </nav>

      <div v-if="current" class="grade-layout">
        <section class="panel stack">
          <div class="row between">
            <b>Câu {{ numberOf(current) }} · {{ formatScore(current.points) }} điểm</b>
            <span class="muted small">{{ current.sectionTitle }}</span>
          </div>
          <hui-markdown :value="current.content" />
          <h3 class="sub">Bài làm của học sinh</h3>
          <hui-markdown v-if="current.answer?.text" class="typed" :value="current.answer.text" />
          <p v-if="!current.answer?.text && !files.length" class="muted">Học sinh bỏ trống câu này.</p>
          <article v-for="f in files" :key="f.file.id" class="page-file">
            <div class="row between small">
              <span>{{ f.file.name }}</span>
              <span class="row">
                <template v-if="f.file.isImage">
                  <hui-button size="sm" variant="outline" @hui-click="annotate(f)">{{ f.annotated ? 'Vẽ thêm' : 'Vẽ nhận xét' }}</hui-button>
                  <hui-button v-if="f.annotated" size="sm" variant="text" @hui-click="clearAnnotation(f)">Bỏ ảnh đã vẽ</hui-button>
                </template>
                <a :href="f.file.url" target="_blank" rel="noopener">Mở {{ f.file.isImage ? 'ảnh gốc' : 'PDF' }}</a>
              </span>
            </div>
            <img v-if="f.file.isImage" :src="(f.annotated ?? f.file).url" :alt="f.file.name" class="page-img" />
            <iframe v-else :src="f.file.url" class="page-pdf" :title="f.file.name" />
            <span v-if="f.changed" class="small changed">Ảnh đã vẽ chưa lưu — bấm "Lưu điểm" để lưu.</span>
          </article>
        </section>

        <aside class="panel stack side">
          <details v-if="current.result?.explanation" class="solution" open>
            <summary><b>Lời giải mẫu</b></summary>
            <hui-markdown :value="current.result.explanation" />
          </details>

          <template v-if="rubric.length">
            <b>Chấm theo bước <span class="muted small">(tổng {{ formatScore(rubricTotal(rubric)) }} đ → quy về {{ formatScore(current.points) }} đ)</span></b>
            <div v-for="(s, i) in rubric" :key="s.id" class="step">
              <div class="step-text"><span class="muted">{{ i + 1 }}.</span> <hui-markdown inline :value="s.content" /></div>
              <div class="step-marks">
                <button v-for="v in quickMarks(s)" :key="v" type="button" class="mark" :class="{ on: Number(marks[s.id] ?? -1) === v }" @click="marks[s.id] = v">
                  {{ v === 0 ? '0' : v === s.points ? 'Đủ' : formatScore(v) }}
                </button>
                <input v-model="marks[s.id]" type="number" min="0" :max="s.points" step="0.05" class="native-input mark-input" :aria-label="`Điểm bước ${i + 1}`" />
                <span class="muted small">/ {{ formatScore(s.points) }}</span>
              </div>
            </div>
            <hui-button size="sm" variant="text" @hui-click="fullMarks">Cho đủ điểm tất cả các bước</hui-button>
          </template>
          <label v-else class="stack small">
            Điểm câu (0 – {{ formatScore(current.points) }})
            <input v-model="score" type="number" min="0" :max="current.points" step="0.25" class="native-input mark-input" />
          </label>
          <p class="preview">Điểm câu: <b>{{ formatScore(preview) }} / {{ formatScore(current.points) }}</b></p>

          <MarkdownEditor v-model="feedback" label="Nhận xét" :rows="4" compact :error="fields.feedback" placeholder="Chỗ làm tốt, chỗ cần sửa…" />
          <CommentSnippets :text="feedback" @insert="insertSnippet" />
          <p v-for="(msg, k) in fieldErrors" :key="k" class="field-error">{{ msg }}</p>
          <hui-alert v-if="formError" severity="error" :description="formError" />
          <div class="row">
            <hui-button :loading="loading" @hui-click="save(true)">Lưu và sang câu / bài tiếp</hui-button>
            <hui-button variant="outline" :loading="loading" @hui-click="save(false)">Lưu điểm</hui-button>
          </div>
          <p class="muted small" style="margin: 0">Chấm xong câu cuối, học sinh nhận thông báo và xem được điểm từng bước, nhận xét, ảnh đã vẽ.</p>
        </aside>
      </div>
    </template>

    <ImageAnnotator v-model:open="annotatorOpen" :src="annotating ? (annotating.annotated ?? annotating.file).url : ''" :name="annotating?.file.name" @saved="onAnnotated" />
  </div>
</template>

<script setup>
/** Giáo viên / trợ giảng chấm các câu tự luận của một bài làm: ảnh bài làm (vẽ nhận xét), lời giải mẫu, điểm từng bước. */
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { attemptApi, teachingApi } from '../../api';
import CommentSnippets from '../../components/teaching/CommentSnippets.vue';
import ImageAnnotator from '../../components/teaching/ImageAnnotator.vue';
import MarkdownEditor from '../../components/teaching/MarkdownEditor.vue';
import { formatDateTime, useSubmit } from '../../utils/form';
import { formatScore, rubricTotal } from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const { loading, error: formError, fields, run } = useSubmit();
const view = ref(null);
const error = ref('');
const current = ref(null);
const marks = ref({});
const score = ref('');
const feedback = ref('');
const files = ref([]);
const annotatorOpen = ref(false);
const annotating = ref(null);

const essays = computed(() => view.value?.items.filter((i) => i.manual) ?? []);
const rubric = computed(() => current.value?.result?.rubric ?? []);
const numberOf = (it) => view.value.items.indexOf(it) + 1;
const fieldErrors = computed(() => ['marks', 'score', 'annotations', 'fileIds'].map((k) => fields[k]).filter(Boolean));

/** Điểm câu theo cách server tính (để giáo viên thấy trước). */
const preview = computed(() => {
  const it = current.value;
  if (!it) return 0;
  const total = rubricTotal(rubric.value);
  if (total <= 0) return Math.min(Math.max(Number(score.value) || 0, 0), it.points);
  const got = rubric.value.reduce((sum, s) => sum + Math.min(Math.max(Number(marks.value[s.id]) || 0, 0), s.points), 0);
  return Math.round((got / total) * it.points * 100) / 100;
});

/** 0 · nửa · đủ (bước nhỏ 0,25 thì chỉ 0 · đủ). */
function quickMarks(s) {
  const half = Math.round((s.points / 2) * 100) / 100;
  return s.points >= 0.5 ? [0, half, s.points] : [0, s.points];
}

function fullMarks() {
  for (const s of rubric.value) marks.value[s.id] = s.points;
}

function select(it) {
  current.value = it;
  const g = it.grading;
  marks.value = { ...(g?.marks ?? {}) };
  score.value = g && !g.pending && !rubric.value.length ? g.score : '';
  feedback.value = g?.feedback ?? '';
  files.value = (it.files ?? []).map((f) => ({ ...f, changed: false }));
  router.replace({ query: { ...route.query, cau: it.position } });
}

function apply(v, position = null) {
  view.value = v;
  const list = v.items.filter((i) => i.manual);
  const pick = list.find((i) => i.position === position) ?? list.find((i) => i.grading?.pending) ?? list[0];
  if (pick) select(pick);
  else current.value = null;
}

async function load() {
  error.value = '';
  view.value = null;
  try {
    const v = await attemptApi.get(route.params.id);
    if (!v.canGrade) {
      error.value = v.status === 'submitted' ? 'Bạn không chấm được bài này.' : 'Học sinh chưa nộp bài.';
      return;
    }
    const q = Number.parseInt(route.query.cau, 10);
    apply(v, Number.isNaN(q) ? null : q);
  } catch (e) {
    error.value = e.message;
  }
}

function insertSnippet(text) {
  feedback.value = feedback.value.trim() ? `${feedback.value.trim()}\n\n${text}` : text;
}

function annotate(f) {
  annotating.value = f;
  annotatorOpen.value = true;
}

function onAnnotated(saved) {
  Object.assign(annotating.value, { annotated: saved, changed: true });
}

function clearAnnotation(f) {
  Object.assign(f, { annotated: null, changed: true });
}

async function save(goNext) {
  const it = current.value;
  const body = {
    marks: Object.fromEntries(
      Object.entries(marks.value)
        .filter(([, v]) => v !== '' && v !== null && v !== undefined)
        .map(([k, v]) => [k, Number(v)]),
    ),
    score: rubric.value.length ? null : score.value === '' ? null : Number(score.value),
    feedback: feedback.value.trim(),
    annotations: files.value.filter((f) => f.annotated).map((f) => ({ fileId: f.file.id, annotatedFileId: f.annotated.id })),
  };
  const res = await run(() => teachingApi.gradeItem(view.value.id, it.position, body));
  if (!res.ok) return;
  message.success(`Đã lưu điểm câu ${numberOf(it)}.`);
  if (!goNext) return apply(res.data, it.position);
  // Câu tự luận chưa chấm kế tiếp trong bài; hết thì sang bài chờ chấm kế tiếp.
  const nextHere = res.data.items.find((i) => i.manual && i.grading?.pending);
  if (nextHere) return apply(res.data, nextHere.position);
  apply(res.data, it.position);
  const queue = await teachingApi.gradingQueue().catch(() => []);
  const next = queue.find((q) => q.attemptId !== view.value.id);
  if (next) router.push({ name: 'teach-grade-attempt', params: { id: next.attemptId } });
  else message.info('Đã chấm hết các bài đang chờ.');
}

watch(() => route.params.id, load, { immediate: true });
</script>

<style scoped>
.score {
  display: grid;
  justify-items: end;
}
.score b {
  font-size: 1.5rem;
}
.tabs {
  flex-wrap: wrap;
  gap: 6px;
}
.tab {
  padding: 6px 12px;
  border: 1px solid var(--hui-border, #d9d9d9);
  border-radius: 999px;
  background: var(--hui-bg-container, #fff);
  font: inherit;
  cursor: pointer;
}
.tab.done {
  border-color: var(--hui-success, #52c41a);
}
.tab.active {
  background: var(--hui-primary, #1677ff);
  border-color: var(--hui-primary, #1677ff);
  color: #fff;
}
.grade-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 380px;
  gap: 16px;
  align-items: start;
}
.side {
  position: sticky;
  top: 72px;
  max-height: calc(100vh - 88px);
  overflow: auto;
}
.sub {
  margin: 8px 0 0;
  font-size: 1rem;
}
.typed {
  padding: 8px 12px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-subtle, #f6f7f9);
}
.page-file {
  display: grid;
  gap: 6px;
}
.page-img {
  width: 100%;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.page-pdf {
  width: 100%;
  height: 70vh;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.changed {
  color: var(--hui-warning, #d48806);
}
.solution {
  padding: 8px 10px;
  border-radius: var(--hui-radius-md, 8px);
  background: color-mix(in srgb, var(--hui-success, #52c41a) 8%, transparent);
}
.step {
  display: grid;
  gap: 4px;
  padding: 6px 0;
  border-bottom: 1px dashed var(--hui-border-secondary, #eceef1);
}
.step-marks {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  align-items: center;
}
.mark {
  min-width: 40px;
  padding: 3px 8px;
  border: 1px solid var(--hui-border, #d9d9d9);
  border-radius: 6px;
  background: var(--hui-bg-container, #fff);
  font: inherit;
  cursor: pointer;
}
.mark.on {
  background: var(--hui-primary, #1677ff);
  border-color: var(--hui-primary, #1677ff);
  color: #fff;
}
.mark-input {
  width: 80px;
}
.preview {
  margin: 0;
}
@media (max-width: 900px) {
  .grade-layout {
    grid-template-columns: 1fr;
  }
  .side {
    position: static;
    max-height: none;
  }
}
</style>

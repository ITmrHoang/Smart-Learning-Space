<template>
  <div class="page">
    <RouterLink :to="backLink" class="small">← Danh sách bài nộp</RouterLink>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="sub">
      <div class="page-header">
        <div class="stack" style="gap: 4px">
          <h1>{{ sub.fullName }} <span class="muted small">@{{ sub.username }}</span></h1>
          <span class="muted small">
            Nộp lần {{ sub.submitCount }} · {{ formatDateTime(sub.submittedAt) }}
            <b v-if="sub.late" class="field-error"> · nộp muộn</b>
          </span>
        </div>
        <hui-tag :color="HOMEWORK_STATUS_COLORS[sub.status]">{{ HOMEWORK_STATUS_LABELS[sub.status] }}</hui-tag>
      </div>

      <div class="grade-layout">
        <section class="panel stack">
          <p v-if="sub.note" class="note small"><b>Lời nhắn của học sinh:</b> {{ sub.note }}</p>
          <article v-for="f in files" :key="f.position" class="page-file">
            <div class="row between small">
              <span>Trang {{ f.position + 1 }} · {{ f.file.name }}</span>
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
            <span v-if="f.changed" class="small changed">Ảnh đã vẽ chưa lưu — bấm "Chấm xong" hoặc "Trả lại" để lưu.</span>
          </article>
        </section>

        <aside class="panel stack side">
          <h2 style="margin: 0">Chấm bài</h2>
          <label v-if="sub.maxScore != null" class="stack small">
            Điểm (thang {{ formatScore(sub.maxScore) }})
            <input v-model="score" type="number" min="0" :max="sub.maxScore" step="0.25" class="native-input score" />
          </label>
          <p v-else class="muted small" style="margin: 0">Bài tập này chỉ nhận xét, không chấm điểm.</p>
          <p v-if="fields.score" class="field-error">{{ fields.score }}</p>
          <MarkdownEditor v-model="feedback" label="Nhận xét" :rows="6" compact :error="fields.feedback" placeholder="Chỗ làm tốt, chỗ cần sửa…" />
          <CommentSnippets :text="feedback" @insert="(t) => (feedback = feedback.trim() ? `${feedback.trim()}\n\n${t}` : t)" />
          <hui-alert v-if="formError" severity="error" :description="formError" />
          <div class="row">
            <hui-button :loading="loading && action === 'grade'" @hui-click="save('grade')">Chấm xong</hui-button>
            <hui-button variant="outline" :loading="loading && action === 'return'" @hui-click="save('return')">Trả lại để làm lại</hui-button>
          </div>
          <p class="muted small" style="margin: 0">Học sinh nhận thông báo ngay. Trả lại: học sinh sửa bài và nộp lại, bước chưa tính là xong.</p>
        </aside>
      </div>
    </template>

    <ImageAnnotator v-model:open="annotatorOpen" :src="annotating ? (annotating.annotated ?? annotating.file).url : ''" :name="annotating?.file.name" @saved="onAnnotated" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import CommentSnippets from '../../components/teaching/CommentSnippets.vue';
import ImageAnnotator from '../../components/teaching/ImageAnnotator.vue';
import MarkdownEditor from '../../components/teaching/MarkdownEditor.vue';
import { formatDateTime, useSubmit } from '../../utils/form';
import { HOMEWORK_STATUS_COLORS, HOMEWORK_STATUS_LABELS, formatScore } from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const { loading, error: formError, fields, run } = useSubmit();
const sub = ref(null);
const files = ref([]);
const error = ref('');
const score = ref('');
const feedback = ref('');
const action = ref('grade');
const annotatorOpen = ref(false);
const annotating = ref(null);

const backLink = computed(() =>
  route.query.classId && route.query.stepId
    ? { name: 'teach-submissions', params: { id: route.query.classId, stepId: route.query.stepId } }
    : sub.value
      ? { name: 'teach-submissions', params: { id: sub.value.classId, stepId: sub.value.stepId } }
      : { name: 'teach-classes' },
);

function apply(s) {
  sub.value = s;
  files.value = s.files.map((f) => ({ ...f, changed: false }));
  score.value = s.score ?? '';
  feedback.value = s.feedback ?? '';
}

async function load() {
  error.value = '';
  try {
    apply(await teachingApi.submission(route.params.id));
  } catch (e) {
    error.value = e.message;
  }
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

async function save(kind) {
  action.value = kind;
  const annotations = files.value.filter((f) => f.changed).map((f) => ({ position: f.position, fileId: f.annotated?.id ?? null }));
  const body = {
    action: kind,
    score: kind === 'grade' && sub.value.maxScore != null && score.value !== '' ? Number(score.value) : null,
    feedback: feedback.value.trim(),
    annotations,
  };
  const res = await run(() => teachingApi.gradeSubmission(sub.value.id, body));
  if (!res.ok) return;
  apply(res.data);
  message.success(kind === 'grade' ? 'Đã chấm bài.' : 'Đã trả lại cho học sinh làm lại.');
  // Chấm tiếp bài chưa chấm của cùng bước.
  const list = await teachingApi.stepSubmissions(sub.value.classId, sub.value.stepId).catch(() => null);
  const next = list?.students.find((s) => s.submission?.status === 'submitted')?.submission.id;
  if (next) router.push({ name: 'teach-submission', params: { id: next }, query: route.query });
}

onMounted(load);
// Sang bài kế (cùng trang, khác id) → tải lại.
watch(
  () => route.params.id,
  (id, old) => id && id !== old && load(),
);
</script>

<style scoped>
.grade-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 340px;
  gap: 16px;
  align-items: start;
}
.side {
  position: sticky;
  top: 72px;
}
.page-file {
  display: grid;
  gap: 6px;
}
.page-img {
  width: 100%;
  border-radius: var(--hui-radius-md, 8px);
  border: 1px solid var(--hui-border-secondary, #eceef1);
}
.page-pdf {
  width: 100%;
  height: 70vh;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.changed {
  color: var(--hui-warning, #d97706);
}
.note {
  margin: 0;
  padding: 8px 10px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-secondary, #f5f6f8);
}
.score {
  width: 120px;
}
@media (max-width: 900px) {
  .grade-layout {
    grid-template-columns: 1fr;
  }
  .side {
    position: static;
  }
}
</style>

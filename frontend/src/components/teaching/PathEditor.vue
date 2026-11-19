<template>
  <div class="stack">
    <div class="row between">
      <div class="small">
        <span v-if="path?.publishedVersion == null" class="field-error">Chưa xuất bản — học sinh chưa thấy lộ trình.</span>
        <span v-else-if="dirty || path.hasChanges">Học sinh đang thấy <b>v{{ path.publishedVersion }}</b>. Có thay đổi chưa xuất bản.</span>
        <span v-else class="muted">Học sinh đang thấy bản mới nhất (v{{ path.publishedVersion }}, {{ formatDateTime(path.publishedAt) }}).</span>
      </div>
      <div class="row">
        <hui-button variant="outline" :loading="saving" :disabled="!dirty" @hui-click="save">Lưu bản nháp</hui-button>
        <hui-button :loading="publishing" @hui-click="publish">Xuất bản</hui-button>
      </div>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />

    <p v-if="!steps.length" class="panel empty muted">Lộ trình trống. Thêm mốc, bài học hoặc đề bên dưới.</p>
    <hui-sortable class="steps" handle=".drag" label="Các bước của lộ trình" @hui-sort="onSort">
      <div v-for="(s, i) in steps" :key="s.key" class="step" :class="`step--${s.kind}`">
        <span class="drag" title="Kéo để đổi thứ tự">⋮⋮</span>
        <span class="num">{{ s.kind === 'milestone' ? '' : stepNumber(i) }}</span>
        <hui-tag size="sm" :color="KIND_COLORS[s.kind]">{{ STEP_KIND_LABELS[s.kind] }}</hui-tag>

        <input v-if="s.kind === 'milestone'" v-model="s.title" class="native-input grow" placeholder="Tên mốc, ví dụ: Tuần 1" @input="dirty = true" />
        <select v-else-if="s.kind === 'lesson'" v-model="s.lessonId" class="native-select grow" @change="dirty = true">
          <option :value="null" disabled>Chọn bài học…</option>
          <option v-for="l in lessons" :key="l.id" :value="l.id">{{ l.title }}{{ l.publishedVersion == null ? ' (chưa xuất bản)' : '' }}</option>
        </select>
        <select v-else-if="s.kind === 'deck'" v-model="s.deckId" class="native-select grow" @change="dirty = true">
          <option :value="null" disabled>Chọn bộ thẻ ghi nhớ…</option>
          <option v-for="d in deckChoices(s)" :key="d.id" :value="d.id">{{ d.title }} · {{ d.cardCount }} thẻ</option>
        </select>
        <select v-else-if="s.kind === 'session'" v-model="s.sessionId" class="native-select grow" @change="dirty = true">
          <option :value="null" disabled>Chọn buổi học của lớp học này…</option>
          <option v-for="x in sessionChoices(s)" :key="x.id" :value="x.id">{{ formatSessionTime(x.startsAt, x.endsAt) }} · {{ x.title }}{{ x.status === 'cancelled' ? ' (đã hủy)' : '' }}</option>
        </select>
        <select v-else-if="s.kind === 'homework'" v-model="s.assignmentId" class="native-select grow" @change="dirty = true">
          <option :value="null" disabled>Chọn bài tập về nhà…</option>
          <option v-for="a in assignmentChoices(s)" :key="a.id" :value="a.id">{{ a.title }}{{ a.maxScore != null ? ` · thang ${a.maxScore}` : ' · chỉ nhận xét' }}</option>
        </select>
        <select v-else v-model="s.examId" class="native-select grow" @change="dirty = true">
          <option :value="null" disabled>Chọn đề…</option>
          <option v-for="e in exams" :key="e.id" :value="e.id">{{ e.title }} · {{ EXAM_KIND_LABELS[e.kind] }} · {{ e.questionCount }} câu</option>
        </select>

        <hui-tag v-if="s.refStatus === 'draft'" size="sm" color="warning">Bài chưa xuất bản</hui-tag>
        <hui-tag v-else-if="s.refStatus === 'empty'" size="sm" color="warning">{{ s.kind === 'deck' ? 'Bộ thẻ chưa có thẻ' : 'Đề chưa có câu' }}</hui-tag>
        <hui-tag v-else-if="s.refStatus === 'archived'" size="sm" color="danger">Đã bị ẩn</hui-tag>
        <hui-tag v-else-if="s.refStatus === 'cancelled'" size="sm" color="danger">Buổi đã hủy</hui-tag>

        <label v-if="s.kind === 'exam'" class="small inline">
          Điểm đạt
          <input v-model.number="s.passScore" type="number" min="0" step="0.25" class="native-input score" placeholder="—" @input="dirty = true" />
        </label>
        <label v-if="s.kind === 'homework' || s.kind === 'exam'" class="small inline">
          {{ s.kind === 'exam' ? 'Hạn làm' : 'Hạn nộp' }}
          <input v-model="s.dueLocal" type="datetime-local" class="native-input" @input="dirty = true" />
        </label>
        <select v-if="s.kind !== 'milestone'" v-model="s.unlock" class="native-select small" @change="dirty = true">
          <option value="free">Mở tự do</option>
          <option value="previous">Xong bước trước mới mở</option>
        </select>
        <details v-if="s.kind !== 'milestone'" class="audience small" :class="{ restricted: isRestricted(s) }">
          <summary>Dành cho: <b>{{ audienceLabel(s) }}</b></summary>
          <div class="audience__body">
            <p class="muted" style="margin: 0">Không chọn ai = cả lớp. Chọn nhóm / học sinh = chỉ những người đó thấy và tính vào tiến độ.</p>
            <div v-if="groups.length" class="audience__list">
              <b>Nhóm</b>
              <label v-for="g in groups" :key="g.id"><input v-model="s.audienceGroupIds" type="checkbox" :value="g.id" @change="dirty = true" /> {{ g.name }} ({{ g.memberCount }})</label>
            </div>
            <div class="audience__list">
              <b>Học sinh</b>
              <label v-for="m in students" :key="m.userId"><input v-model="s.audienceUserIds" type="checkbox" :value="m.userId" @change="dirty = true" /> {{ m.fullName }}</label>
              <span v-if="!students.length" class="muted">Lớp học chưa có học sinh.</span>
            </div>
          </div>
        </details>
        <span class="row tools">
          <button :disabled="i === 0" title="Lên" @click="move(i, -1)">↑</button>
          <button :disabled="i === steps.length - 1" title="Xuống" @click="move(i, 1)">↓</button>
          <button title="Xóa bước" @click="remove(i)">✕</button>
        </span>
      </div>
    </hui-sortable>

    <div class="row">
      <hui-button size="sm" variant="outline" @hui-click="add('milestone')">+ Mốc</hui-button>
      <hui-button size="sm" variant="outline" @hui-click="add('lesson')">+ Bài học</hui-button>
      <hui-button size="sm" variant="outline" @hui-click="add('exam')">+ Đề ôn tập / kiểm tra</hui-button>
      <hui-button size="sm" variant="outline" @hui-click="add('session')">+ Buổi học</hui-button>
      <hui-button size="sm" variant="outline" @hui-click="add('deck')">+ Thẻ ghi nhớ</hui-button>
      <hui-button size="sm" variant="outline" @hui-click="add('homework')">+ Bài tập về nhà</hui-button>
      <span class="muted small">
        Chưa có bài? <RouterLink :to="{ name: 'teach-lesson', params: { id: 'moi' } }">Soạn bài</RouterLink> ·
        <RouterLink :to="{ name: 'teach-exam', params: { id: 'moi' } }">Tạo đề</RouterLink> ·
        <RouterLink :to="{ name: 'teach-schedule', query: { classId } }">Xếp lịch buổi học</RouterLink> ·
        <RouterLink :to="{ name: 'teach-deck', params: { id: 'moi' } }">Tạo bộ thẻ</RouterLink> ·
        <RouterLink :to="{ name: 'teach-assignments' }">Soạn bài tập</RouterLink>
      </span>
    </div>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { formatSessionTime, fromLocal, localParts } from '../../utils/schedule';
import { EXAM_KIND_LABELS, STEP_KIND_LABELS } from '../../utils/teaching';

const KIND_COLORS = { exam: 'primary', lesson: 'success', session: 'warning', deck: 'info', homework: 'danger', milestone: 'secondary' };

const DAY = 86_400_000;
const props = defineProps({ classId: { type: String, required: true } });
const emit = defineEmits(['published']);

const path = ref(null);
const steps = ref([]);
const lessons = ref([]);
const exams = ref([]);
/** Buổi học của lớp học này (90 ngày tới, kèm vài ngày trước) */
const sessions = ref([]);
const decks = ref([]);
const assignments = ref([]);
/** Nhóm học sinh của giáo viên + học sinh đang học lớp học này (chọn "Dành cho") */
const groups = ref([]);
const students = ref([]);
const dirty = ref(false);
const saving = ref(false);
const publishing = ref(false);
const error = ref('');
let seq = 0;

/** ISO → giá trị ô datetime-local (giờ của máy). */
function toLocalInput(iso) {
  if (!iso) return '';
  const { date, time } = localParts(iso);
  return `${date}T${time}`;
}

function apply(p) {
  path.value = p;
  steps.value = p.steps.map((s) => ({
    ...s,
    key: ++seq,
    dueLocal: toLocalInput(s.dueAt),
    audienceGroupIds: [...(s.audienceGroupIds ?? [])],
    audienceUserIds: [...(s.audienceUserIds ?? [])],
  }));
  dirty.value = false;
}

function stepNumber(i) {
  return steps.value.slice(0, i + 1).filter((s) => s.kind !== 'milestone').length;
}

function add(kind) {
  steps.value.push({
    key: ++seq,
    id: null,
    kind,
    title: '',
    lessonId: null,
    examId: null,
    sessionId: null,
    deckId: null,
    assignmentId: null,
    dueLocal: '',
    unlock: 'free',
    passScore: null,
    refStatus: null,
    audienceGroupIds: [],
    audienceUserIds: [],
  });
  dirty.value = true;
}

/** Bài tập chọn được + bài tập bước đang trỏ tới (có thể đã bị ẩn). */
function assignmentChoices(step) {
  const list = [...assignments.value];
  if (step.assignmentId && !list.some((a) => a.id === step.assignmentId)) {
    list.unshift({ id: step.assignmentId, title: step.refTitle ?? 'Bài tập', maxScore: null });
  }
  return list;
}

const isRestricted = (s) => s.audienceGroupIds.length > 0 || s.audienceUserIds.length > 0;

/** "Cả lớp" · "Nhóm nâng cao" · "2 nhóm, 1 học sinh" */
function audienceLabel(s) {
  if (!isRestricted(s)) return 'Cả lớp';
  const parts = [];
  if (s.audienceGroupIds.length === 1 && !s.audienceUserIds.length) {
    return groups.value.find((g) => g.id === s.audienceGroupIds[0])?.name ?? '1 nhóm';
  }
  if (s.audienceGroupIds.length) parts.push(`${s.audienceGroupIds.length} nhóm`);
  if (s.audienceUserIds.length) parts.push(`${s.audienceUserIds.length} học sinh`);
  return parts.join(', ');
}

/** Buổi học chọn được: buổi của lớp học trong khoảng đã tải + buổi bước đang trỏ tới (có thể đã qua). */
/** Bộ thẻ chọn được + bộ thẻ bước đang trỏ tới (có thể đã bị ẩn). */
function deckChoices(step) {
  const list = [...decks.value];
  if (step.deckId && !list.some((d) => d.id === step.deckId)) {
    list.unshift({ id: step.deckId, title: step.refTitle ?? 'Bộ thẻ', cardCount: step.itemCount ?? 0 });
  }
  return list;
}

function sessionChoices(step) {
  const list = [...sessions.value];
  if (step.sessionId && !list.some((x) => x.id === step.sessionId)) {
    list.unshift({ id: step.sessionId, title: step.refTitle ?? 'Buổi học', startsAt: step.startsAt, endsAt: step.startsAt, status: step.refStatus });
  }
  return list;
}

function move(i, d) {
  const [x] = steps.value.splice(i, 1);
  steps.value.splice(i + d, 0, x);
  dirty.value = true;
}

function remove(i) {
  steps.value.splice(i, 1);
  dirty.value = true;
}

/** Kéo thả: hủy việc hui-sortable tự dời DOM, đổi thứ tự trong mảng để Vue vẽ lại. */
function onSort(e) {
  e.preventDefault();
  const { oldIndex, newIndex } = e.detail;
  if (oldIndex === newIndex) return;
  const [x] = steps.value.splice(oldIndex, 1);
  steps.value.splice(newIndex, 0, x);
  dirty.value = true;
}

function payload() {
  return steps.value.map((s) => ({
    id: s.id,
    kind: s.kind,
    title: s.title ?? '',
    lessonId: s.kind === 'lesson' ? s.lessonId : null,
    examId: s.kind === 'exam' ? s.examId : null,
    sessionId: s.kind === 'session' ? s.sessionId : null,
    deckId: s.kind === 'deck' ? s.deckId : null,
    assignmentId: s.kind === 'homework' ? s.assignmentId : null,
    dueAt: (s.kind === 'homework' || s.kind === 'exam') && s.dueLocal ? fromLocal(...s.dueLocal.split('T')) : null,
    audienceGroupIds: s.kind === 'milestone' ? [] : s.audienceGroupIds,
    audienceUserIds: s.kind === 'milestone' ? [] : s.audienceUserIds,
    unlock: s.unlock,
    passScore: s.kind === 'exam' && s.passScore !== '' && s.passScore != null ? Number(s.passScore) : null,
  }));
}

async function save() {
  saving.value = true;
  error.value = '';
  try {
    apply(await teachingApi.savePath(props.classId, payload()));
    message.success('Đã lưu bản nháp lộ trình.');
    return true;
  } catch (e) {
    error.value = e.fields ? Object.values(e.fields)[0][0] : e.message;
    return false;
  } finally {
    saving.value = false;
  }
}

async function publish() {
  if (dirty.value && !(await save())) return;
  publishing.value = true;
  error.value = '';
  try {
    apply(await teachingApi.publishPath(props.classId));
    message.success('Đã xuất bản lộ trình. Học sinh thấy bản mới, tiến độ cũ được giữ.');
    emit('published');
  } catch (e) {
    error.value = e.message;
  } finally {
    publishing.value = false;
  }
}

onMounted(async () => {
  const now = Date.now();
  const [p, ls, es, ss, ds, as, gs, ms] = await Promise.all([
    teachingApi.path(props.classId),
    teachingApi.lessons({ pageSize: 100 }).catch(() => ({ items: [] })),
    teachingApi.exams({ pageSize: 100 }).catch(() => ({ items: [] })),
    teachingApi
      .sessions({ classId: props.classId, from: new Date(now - 3 * DAY).toISOString(), to: new Date(now + 90 * DAY).toISOString() })
      .catch(() => []),
    teachingApi.decks().catch(() => []),
    teachingApi.assignments().catch(() => []),
    teachingApi.groups().catch(() => []),
    teachingApi.members(props.classId, 'active').catch(() => []),
  ]);
  assignments.value = as;
  groups.value = gs;
  students.value = ms;
  lessons.value = ls.items;
  exams.value = es.items;
  sessions.value = ss;
  decks.value = ds;
  apply(p);
});
</script>

<style scoped>
.steps {
  display: grid;
  gap: 8px;
}
.step {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-container, #fff);
}
.step--milestone {
  background: var(--hui-bg-secondary, #f5f6f8);
  font-weight: 600;
}
.drag {
  cursor: grab;
  color: var(--hui-text-secondary, #6b7280);
  user-select: none;
  letter-spacing: -2px;
}
.num {
  min-width: 1.5em;
  text-align: right;
  color: var(--hui-text-secondary, #6b7280);
}
.grow {
  flex: 1 1 240px;
}
.inline {
  display: inline-flex;
  gap: 6px;
  align-items: center;
}
.score {
  width: 80px;
}
.audience {
  flex-basis: 100%;
}
.audience summary {
  cursor: pointer;
  color: var(--hui-text-secondary, #6b7280);
}
.audience.restricted summary b {
  color: var(--hui-warning, #d97706);
}
.audience__body {
  display: grid;
  gap: 8px;
  margin-top: 6px;
  padding: 8px 10px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-secondary, #f5f6f8);
}
.audience__list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  align-items: center;
}
.tools button {
  border: 0;
  background: none;
  cursor: pointer;
  color: var(--hui-text-secondary, #6b7280);
}
.tools button:disabled {
  opacity: 0.3;
}
</style>

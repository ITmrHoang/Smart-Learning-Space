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
        <hui-tag size="sm" :color="s.kind === 'exam' ? 'primary' : s.kind === 'lesson' ? 'success' : 'secondary'">{{ STEP_KIND_LABELS[s.kind] }}</hui-tag>

        <input v-if="s.kind === 'milestone'" v-model="s.title" class="native-input grow" placeholder="Tên mốc, ví dụ: Tuần 1" @input="dirty = true" />
        <select v-else-if="s.kind === 'lesson'" v-model="s.lessonId" class="native-select grow" @change="dirty = true">
          <option :value="null" disabled>Chọn bài học…</option>
          <option v-for="l in lessons" :key="l.id" :value="l.id">{{ l.title }}{{ l.publishedVersion == null ? ' (chưa xuất bản)' : '' }}</option>
        </select>
        <select v-else v-model="s.examId" class="native-select grow" @change="dirty = true">
          <option :value="null" disabled>Chọn đề…</option>
          <option v-for="e in exams" :key="e.id" :value="e.id">{{ e.title }} · {{ EXAM_KIND_LABELS[e.kind] }} · {{ e.questionCount }} câu</option>
        </select>

        <hui-tag v-if="s.refStatus === 'draft'" size="sm" color="warning">Bài chưa xuất bản</hui-tag>
        <hui-tag v-else-if="s.refStatus === 'empty'" size="sm" color="warning">Đề chưa có câu</hui-tag>
        <hui-tag v-else-if="s.refStatus === 'archived'" size="sm" color="danger">Đã bị ẩn</hui-tag>

        <label v-if="s.kind === 'exam'" class="small inline">
          Điểm đạt
          <input v-model.number="s.passScore" type="number" min="0" step="0.25" class="native-input score" placeholder="—" @input="dirty = true" />
        </label>
        <select v-if="s.kind !== 'milestone'" v-model="s.unlock" class="native-select small" @change="dirty = true">
          <option value="free">Mở tự do</option>
          <option value="previous">Xong bước trước mới mở</option>
        </select>
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
      <span class="muted small">
        Chưa có bài? <RouterLink :to="{ name: 'teach-lesson', params: { id: 'moi' } }">Soạn bài</RouterLink> ·
        <RouterLink :to="{ name: 'teach-exam', params: { id: 'moi' } }">Tạo đề</RouterLink>
      </span>
    </div>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, ref } from 'vue';

import { teachingApi } from '../../api';
import { formatDateTime } from '../../utils/form';
import { EXAM_KIND_LABELS, STEP_KIND_LABELS } from '../../utils/teaching';

const props = defineProps({ classId: { type: String, required: true } });
const emit = defineEmits(['published']);

const path = ref(null);
const steps = ref([]);
const lessons = ref([]);
const exams = ref([]);
const dirty = ref(false);
const saving = ref(false);
const publishing = ref(false);
const error = ref('');
let seq = 0;

function apply(p) {
  path.value = p;
  steps.value = p.steps.map((s) => ({ ...s, key: ++seq }));
  dirty.value = false;
}

function stepNumber(i) {
  return steps.value.slice(0, i + 1).filter((s) => s.kind !== 'milestone').length;
}

function add(kind) {
  steps.value.push({ key: ++seq, id: null, kind, title: '', lessonId: null, examId: null, unlock: 'free', passScore: null, refStatus: null });
  dirty.value = true;
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
  const [p, ls, es] = await Promise.all([
    teachingApi.path(props.classId),
    teachingApi.lessons({ pageSize: 100 }).catch(() => ({ items: [] })),
    teachingApi.exams({ pageSize: 100 }).catch(() => ({ items: [] })),
  ]);
  lessons.value = ls.items;
  exams.value = es.items;
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

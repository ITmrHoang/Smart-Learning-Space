<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <h1>Bộ đề</h1>
        <span class="muted small">Gom các đề thành bộ ("20 đề ôn thi vào 10"), sắp thứ tự, rồi giao nhanh từng đề cho lớp / nhóm.</span>
      </div>
      <hui-button @hui-click="openSet(null)">Tạo bộ đề</hui-button>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />

    <div class="layout">
      <aside class="panel list">
        <p v-if="loaded && !sets.length" class="muted small">Chưa có bộ đề nào.</p>
        <button v-for="s in sets" :key="s.id" type="button" class="set" :class="{ active: current?.id === s.id }" @click="openSet(s.id)">
          <b>{{ s.title }}</b>
          <span class="muted small">{{ subjectName(s.subjectId) }} · {{ s.examCount }} đề</span>
        </button>
      </aside>

      <section v-if="editing" class="panel stack">
        <div class="grid-2">
          <hui-input v-model="form.title" label="Tên bộ đề" placeholder="20 đề ôn thi vào 10 — 2026" :error="fields.title" required />
          <hui-select label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="onSubject($event.detail.value)" />
        </div>
        <hui-textarea label="Mô tả (không bắt buộc)" :rows="2" :value="form.description" @input="form.description = $event.detail.value" />

        <div class="row between">
          <b>Các đề ({{ form.exams.length }})</b>
          <span class="row">
            <select v-model="adding" class="native-select" :disabled="!available.length">
              <option value="">{{ available.length ? '— Thêm đề vào bộ —' : 'Không còn đề nào của môn này' }}</option>
              <option v-for="e in available" :key="e.id" :value="e.id">{{ e.title }}</option>
            </select>
            <hui-button size="sm" variant="outline" :disabled="!adding" @hui-click="addExam">Thêm</hui-button>
          </span>
        </div>
        <p v-if="fields.examIds" class="field-error">{{ fields.examIds }}</p>
        <ol v-if="form.exams.length" class="exams">
          <li v-for="(e, i) in form.exams" :key="e.id">
            <span class="grow">
              <b>{{ e.title }}</b>
              <span class="muted small"> · {{ EXAM_KIND_LABELS[e.kind] }} · {{ e.questionCount }} câu{{ e.timeLimitMinutes ? ` · ${e.timeLimitMinutes} phút` : '' }}</span>
            </span>
            <span class="row">
              <RouterLink :to="{ name: 'teach-exam', params: { id: e.id } }" class="small">Mở</RouterLink>
              <RouterLink :to="{ name: 'teach-exam-papers', params: { id: e.id } }" class="small">Trộn mã</RouterLink>
              <hui-button size="sm" variant="outline" :disabled="!current" @hui-click="assigning = e">Giao nhanh</hui-button>
              <button class="icon" :disabled="i === 0" title="Lên" @click="move(i, -1)">↑</button>
              <button class="icon" :disabled="i === form.exams.length - 1" title="Xuống" @click="move(i, 1)">↓</button>
              <button class="icon" title="Bỏ khỏi bộ" @click="form.exams.splice(i, 1)">✕</button>
            </span>
          </li>
        </ol>
        <p v-else class="muted small">Chưa có đề nào trong bộ.</p>
        <hui-alert v-if="formError" severity="error" :description="formError" />
        <div class="row between">
          <hui-button v-if="current" variant="text" color="danger" @hui-click="archive">Xóa bộ đề</hui-button>
          <span v-else />
          <hui-button :loading="loading" @hui-click="save">{{ current ? 'Lưu bộ đề' : 'Tạo bộ đề' }}</hui-button>
        </div>
      </section>
      <section v-else class="panel muted">Chọn một bộ đề bên trái, hoặc bấm "Tạo bộ đề".</section>
    </div>

    <QuickAssignModal :open="Boolean(assigning)" kind="exam" :refId="assigning?.id ?? ''" :refTitle="assigning?.title ?? ''" @update:open="(v) => !v && (assigning = null)" />
  </div>
</template>

<script setup>
/** Bộ đề (M41): danh sách bên trái, soạn bộ đề bên phải; giao nhanh từng đề. */
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import QuickAssignModal from '../../components/teaching/QuickAssignModal.vue';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { EXAM_KIND_LABELS } from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const { loading, error: formError, fields, run, reset } = useSubmit();
const sets = ref([]);
const loaded = ref(false);
const error = ref('');
const current = ref(null);
const editing = ref(false);
const exams = ref([]);
const adding = ref('');
const assigning = ref(null);
const form = reactive({ subjectId: null, title: '', description: '', exams: [] });

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const subjectName = (id) => master.value?.subjects.find((s) => s.id === id)?.name ?? '';
const available = computed(() => exams.value.filter((e) => !form.exams.some((x) => x.id === e.id)));

async function loadSets() {
  try {
    sets.value = await teachingApi.examSets();
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
}

async function loadExams(subjectId) {
  exams.value = subjectId ? (await teachingApi.exams({ subjectId, pageSize: 100 }).catch(() => ({ items: [] }))).items : [];
}

async function openSet(id) {
  reset();
  adding.value = '';
  if (!id) {
    current.value = null;
    Object.assign(form, { subjectId: master.value?.subjects[0]?.id ?? null, title: '', description: '', exams: [] });
  } else {
    try {
      current.value = await teachingApi.examSet(id);
    } catch (e) {
      message.error(e.message);
      return;
    }
    Object.assign(form, {
      subjectId: current.value.subjectId,
      title: current.value.title,
      description: current.value.description,
      exams: [...current.value.exams],
    });
  }
  editing.value = true;
  router.replace({ query: id ? { bo: id } : {} });
  loadExams(form.subjectId);
}

function onSubject(id) {
  if (form.exams.length && !window.confirm('Đổi môn sẽ bỏ các đề đã chọn. Tiếp tục?')) return;
  form.subjectId = id;
  form.exams = [];
  loadExams(id);
}

function addExam() {
  const e = exams.value.find((x) => x.id === adding.value);
  if (e) form.exams.push({ id: e.id, title: e.title, kind: e.kind, questionCount: e.questionCount, timeLimitMinutes: e.timeLimitMinutes });
  adding.value = '';
}

function move(i, d) {
  const [x] = form.exams.splice(i, 1);
  form.exams.splice(i + d, 0, x);
}

async function save() {
  const body = { subjectId: form.subjectId, title: form.title, description: form.description, examIds: form.exams.map((e) => e.id) };
  const res = await run(() => (current.value ? teachingApi.updateExamSet(current.value.id, body) : teachingApi.createExamSet(body)));
  if (!res.ok) return;
  message.success(current.value ? 'Đã lưu bộ đề.' : 'Đã tạo bộ đề.');
  await loadSets();
  openSet(res.data.id);
}

async function archive() {
  if (!window.confirm(`Xóa bộ đề "${current.value.title}"? Các đề trong bộ vẫn giữ nguyên.`)) return;
  try {
    await teachingApi.archiveExamSet(current.value.id);
    message.success('Đã xóa bộ đề.');
    editing.value = false;
    current.value = null;
    router.replace({ query: {} });
    loadSets();
  } catch (e) {
    message.error(e.message);
  }
}

onMounted(async () => {
  await loadMaster();
  await loadSets();
  if (route.query.bo) openSet(route.query.bo);
});
</script>

<style scoped>
.layout {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}
.list {
  display: grid;
  gap: 6px;
}
.set {
  display: grid;
  gap: 2px;
  text-align: left;
  padding: 8px 10px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-container, #fff);
  font: inherit;
  cursor: pointer;
}
.set.active {
  border-color: var(--hui-primary, #1677ff);
  background: color-mix(in srgb, var(--hui-primary, #1677ff) 8%, transparent);
}
.exams {
  margin: 0;
  padding-left: 1.4em;
  display: grid;
  gap: 6px;
}
.exams li {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
  padding: 6px 8px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.grow {
  flex: 1 1 240px;
}
@media (max-width: 800px) {
  .layout {
    grid-template-columns: 1fr;
  }
}
</style>

<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <RouterLink to="/day-hoc/giao-an" class="small">← Giáo án</RouterLink>
        <h1>{{ form.title || 'Giáo án' }}</h1>
      </div>
      <div v-if="plan" class="row">
        <span class="muted small" :class="{ 'field-error': saveState === 'error' }">{{ saveText }}</span>
        <RouterLink :to="{ name: 'teach-plan-print', params: { id: plan.id } }" target="_blank" class="small">Xem bản in / Lưu PDF</RouterLink>
        <hui-button variant="outline" :loading="exporting" @hui-click="exportWord">Tải Word</hui-button>
      </div>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />

    <template v-if="plan">
      <section class="panel form-grid">
        <hui-input v-model="form.title" label="Tên bài dạy" :error="fields.title" required />
        <div class="grid-3">
          <hui-select label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="form.subjectId = $event.detail.value" />
          <hui-select label="Lớp (không bắt buộc)" allow-clear :options="gradeOptions" :value="form.gradeId" @change="form.gradeId = $event.detail.value || null" />
          <hui-select label="Mẫu" :options="TEMPLATE_OPTIONS" :value="form.template" @change="form.template = $event.detail.value" />
        </div>
        <details class="info">
          <summary class="small"><b>Thông tin chung</b> (trường, tổ, giáo viên, ngày soạn / ngày dạy, lớp, số tiết)</summary>
          <div class="grid-3">
            <hui-input v-for="[key, label] in INFO_FIELDS" :key="key" v-model="form.info[key]" :label="label" />
          </div>
        </details>
        <hui-select
          label="Bài học cho học sinh đi kèm (không bắt buộc)"
          allow-clear
          :options="lessonOptions"
          :value="form.lessonId"
          :error="fields.lessonId"
          @change="form.lessonId = $event.detail.value || null"
        />
        <AttachmentList v-model="form.attachments" editable label="Tài liệu kèm theo (phiếu học tập, bài trình chiếu…)" />
        <p v-if="fields.attachmentIds" class="field-error">{{ fields.attachmentIds }}</p>
      </section>

      <p v-if="fields.blocks" class="field-error">{{ fields.blocks }}</p>
      <section v-for="(b, i) in form.blocks" :key="b.id" class="panel block">
        <div class="block-head">
          <hui-tag size="sm" :color="KIND_COLORS[b.kind]">{{ BLOCK_KINDS[b.kind] }}</hui-tag>
          <input v-model="b.title" class="title-input" :placeholder="b.kind === 'activity' ? 'Hoạt động …' : 'Tiêu đề mục'" />
          <label v-if="b.kind === 'activity'" class="minutes small">
            <input v-model.number="b.minutes" type="number" min="0" max="600" /> phút
          </label>
          <span class="row actions">
            <hui-button size="sm" variant="text" :disabled="i === 0" title="Lên trên" @hui-click="move(i, -1)">↑</hui-button>
            <hui-button size="sm" variant="text" :disabled="i === form.blocks.length - 1" title="Xuống dưới" @hui-click="move(i, 1)">↓</hui-button>
            <hui-button size="sm" variant="text" title="Xóa mục" @hui-click="remove(i)">✕</hui-button>
          </span>
        </div>

        <MarkdownEditor v-if="b.kind === 'section'" v-model="b.body" label="Nội dung" :rows="5" compact />

        <template v-else-if="b.kind === 'activity'">
          <MarkdownEditor v-for="[key, label] in ACTIVITY_PARTS" :key="key" v-model="b[key]" :label="label" :rows="2" compact />
          <p class="small"><b>d) Tổ chức thực hiện</b></p>
          <MarkdownEditor v-for="[key, label] in ACTIVITY_STEPS" :key="key" v-model="b.steps[key]" :label="label" :rows="2" compact />
        </template>

        <template v-else-if="b.kind === 'two_columns'">
          <div v-for="(r, ri) in b.rows" :key="ri" class="two-cols">
            <MarkdownEditor v-model="r.teacher" label="Hoạt động của giáo viên" :rows="3" compact />
            <MarkdownEditor v-model="r.student" label="Hoạt động của học sinh" :rows="3" compact />
            <hui-button size="sm" variant="text" title="Xóa dòng" @hui-click="b.rows.splice(ri, 1)">✕</hui-button>
          </div>
          <hui-button size="sm" variant="outline" :disabled="b.rows.length >= 60" @hui-click="b.rows.push({ teacher: '', student: '' })">Thêm dòng</hui-button>
        </template>

        <template v-else-if="b.kind === 'problem_type'">
          <MarkdownEditor v-for="[key, label] in PROBLEM_PARTS" :key="key" v-model="b[key]" :label="label" :rows="3" compact />
        </template>

        <template v-else-if="b.kind === 'questions'">
          <div class="row">
            <hui-button size="sm" variant="outline" @hui-click="openPicker(i)">Thêm câu từ ngân hàng</hui-button>
            <label class="small"><input v-model="b.showAnswers" type="checkbox" /> In đáp án, lời giải ở phụ lục</label>
          </div>
          <p v-if="!b.questionIds.length" class="muted small">Chưa có câu hỏi.</p>
          <ol class="questions">
            <li v-for="id in b.questionIds" :key="id">
              <div v-if="questions.get(id)" class="question">
                <QuestionView :item="questionAsItem(questions.get(id), { withContent: true })" :number="1" readonly bare />
              </div>
              <span v-else class="muted small">Đang tải câu hỏi…</span>
              <hui-button size="sm" variant="text" @hui-click="b.questionIds = b.questionIds.filter((x) => x !== id)">Bỏ câu này</hui-button>
            </li>
          </ol>
        </template>
      </section>

      <section class="panel row add">
        <span class="small"><b>Thêm mục:</b></span>
        <hui-button v-for="(label, kind) in BLOCK_KINDS" :key="kind" size="sm" variant="outline" :disabled="form.blocks.length >= 80" @hui-click="add(kind)">
          {{ label }}
        </hui-button>
        <span class="muted small">Tổng thời lượng các hoạt động: {{ totalMinutes(form.blocks) }} phút</span>
      </section>
    </template>

    <QuestionPickerModal v-model:open="picker.open" :subjectId="form.subjectId" :exclude="pickerExclude" @picked="onPicked" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue';
import { onBeforeRouteLeave, useRoute } from 'vue-router';

import { teachingApi } from '../../api';
import AttachmentList from '../../components/teaching/AttachmentList.vue';
import MarkdownEditor from '../../components/teaching/MarkdownEditor.vue';
import QuestionPickerModal from '../../components/teaching/QuestionPickerModal.vue';
import QuestionView from '../../components/teaching/QuestionView.vue';
import { downloadBlob, exportDocx } from '../../utils/docx/document';
import {
  ACTIVITY_PARTS,
  ACTIVITY_STEPS,
  BLOCK_KINDS,
  INFO_FIELDS,
  PROBLEM_PARTS,
  TEMPLATE_OPTIONS,
  docxFileName,
  moveBlock,
  newBlock,
  planPayload,
  totalMinutes,
} from '../../utils/lessonPlan';
import { loadPlanQuestions, renderPlanDom } from '../../utils/lessonPlanRender';
import { useMasterData } from '../../utils/masterData';
import { questionAsItem } from '../../utils/teaching';

const KIND_COLORS = { section: 'secondary', activity: 'primary', two_columns: 'info', problem_type: 'warning', questions: 'success' };
const AUTOSAVE_MS = 2000;

const route = useRoute();
const { data: master, load: loadMaster } = useMasterData();
const plan = ref(null);
const error = ref('');
const fields = ref({});
const form = reactive({ title: '', subjectId: null, gradeId: null, template: 'cv5512', info: {}, blocks: [], lessonId: null, attachments: [] });
const lessons = ref([]);
const questions = reactive(new Map());
const picker = reactive({ open: false, block: 0 });
const saveState = ref('saved');
const savedAt = ref(null);
const exporting = ref(false);
let timer = null;
let baseline = '';

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ value: g.id, label: g.name })));
const lessonOptions = computed(() => lessons.value.map((l) => ({ value: l.id, label: l.title })));
const pickerExclude = computed(() => form.blocks[picker.block]?.questionIds ?? []);
const saveText = computed(
  () =>
    ({
      saved: savedAt.value ? `Đã lưu lúc ${savedAt.value.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' })}` : 'Đã lưu',
      dirty: 'Chưa lưu…',
      saving: 'Đang lưu…',
      error: 'Chưa lưu được — kiểm tra lỗi bên dưới',
    })[saveState.value],
);

function apply(p) {
  plan.value = p;
  Object.assign(form, {
    title: p.title,
    subjectId: p.subjectId,
    gradeId: p.gradeId,
    template: p.template,
    info: { ...p.info },
    blocks: p.blocks.map((b) => (b.kind === 'activity' ? { ...b, steps: { assign: '', perform: '', report: '', conclude: '', ...b.steps } } : { ...b })),
    lessonId: p.lessonId,
    attachments: p.attachments ?? [],
  });
  baseline = JSON.stringify(planPayload(form));
}

async function save() {
  clearTimeout(timer);
  const body = planPayload(form);
  const snapshot = JSON.stringify(body);
  if (snapshot === baseline || !form.title.trim()) return true;
  saveState.value = 'saving';
  try {
    const saved = await teachingApi.updateLessonPlan(plan.value.id, body);
    plan.value = saved;
    baseline = snapshot;
    fields.value = {};
    savedAt.value = new Date();
    saveState.value = JSON.stringify(planPayload(form)) === baseline ? 'saved' : 'dirty';
    return true;
  } catch (e) {
    fields.value = Object.fromEntries(Object.entries(e.fields ?? {}).map(([k, v]) => [k, v[0]]));
    if (!e.fields) message.error(e.message);
    saveState.value = 'error';
    return false;
  }
}

// Tự lưu sau khi ngừng gõ
watch(
  () => JSON.stringify(planPayload(form)),
  (now) => {
    if (!plan.value || now === baseline) return;
    saveState.value = 'dirty';
    clearTimeout(timer);
    timer = setTimeout(save, AUTOSAVE_MS);
  },
);

function add(kind) {
  form.blocks.push(newBlock(kind));
}

function move(i, dir) {
  form.blocks = moveBlock(form.blocks, i, dir);
}

function remove(i) {
  const b = form.blocks[i];
  const hasText = JSON.stringify(b).replace(/"(id|kind)":"[^"]*"/g, '').match(/"[^"]+":"[^"]+"/);
  if (hasText && !window.confirm(`Xóa mục "${b.title || BLOCK_KINDS[b.kind]}"?`)) return;
  form.blocks.splice(i, 1);
}

function openPicker(i) {
  picker.block = i;
  picker.open = true;
}

async function onPicked(list) {
  const block = form.blocks[picker.block];
  if (!block) return;
  for (const q of list) if (!block.questionIds.includes(q.id)) block.questionIds.push(q.id);
  await loadPlanQuestions({ blocks: [block] }, teachingApi.question, questions);
}

async function exportWord() {
  exporting.value = true;
  try {
    await save();
    const current = { ...plan.value, ...planPayload(form), info: form.info, blocks: form.blocks, attachments: form.attachments };
    await loadPlanQuestions(current, teachingApi.question, questions);
    const root = renderPlanDom(current, {
      subjectName: master.value?.subjects.find((s) => s.id === form.subjectId)?.name ?? '',
      gradeName: master.value?.grades.find((g) => g.id === form.gradeId)?.name ?? '',
      questions,
    });
    downloadBlob(await exportDocx({ title: form.title, root }), docxFileName(form.title));
  } catch (e) {
    message.error(`Không xuất được file Word: ${e.message}`);
  } finally {
    exporting.value = false;
  }
}

function beforeUnload(e) {
  if (saveState.value === 'dirty' || saveState.value === 'saving') {
    e.preventDefault();
    e.returnValue = '';
  }
}

onBeforeRouteLeave(async () => {
  if (saveState.value === 'dirty') await save();
  return true;
});

onMounted(async () => {
  window.addEventListener('beforeunload', beforeUnload);
  await loadMaster();
  try {
    apply(await teachingApi.lessonPlan(route.params.id));
  } catch (e) {
    error.value = e.message;
    return;
  }
  loadPlanQuestions(plan.value, teachingApi.question, questions);
  lessons.value = (await teachingApi.lessons({ page: 1, pageSize: 100 }).catch(() => ({ items: [] }))).items ?? [];
});

onBeforeUnmount(() => {
  window.removeEventListener('beforeunload', beforeUnload);
  clearTimeout(timer);
});
</script>

<style scoped>
.block {
  display: grid;
  gap: 10px;
}
.block-head {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-wrap: wrap;
}
.title-input {
  flex: 1;
  min-width: 200px;
  padding: 6px 8px;
  border: 1px solid var(--hui-border-secondary, #e5e7eb);
  border-radius: var(--hui-radius-sm, 6px);
  font: inherit;
  font-weight: 600;
}
.minutes input {
  width: 64px;
  padding: 4px 6px;
}
.actions {
  gap: 0;
}
.two-cols {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 8px;
  align-items: start;
}
.questions {
  display: grid;
  gap: 8px;
  padding-left: 20px;
}
.question {
  padding: 8px;
  border: 1px solid var(--hui-border-secondary, #e5e7eb);
  border-radius: var(--hui-radius-sm, 6px);
}
.add {
  flex-wrap: wrap;
}
.info summary {
  cursor: pointer;
  margin-bottom: 8px;
}
@media (max-width: 700px) {
  .two-cols {
    grid-template-columns: 1fr;
  }
}
</style>

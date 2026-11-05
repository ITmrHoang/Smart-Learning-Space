<template>
  <hui-modal
    :open="open"
    :title="question ? `Sửa câu hỏi (phiên bản ${question.version})` : 'Thêm câu hỏi'"
    size="lg"
    ok-text="Lưu"
    cancel-text="Hủy"
    :confirmLoading="loading"
    @hui-ok="onOk"
    @hui-close="close"
  >
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-alert
        v-if="question && question.usedCount > 0"
        severity="info"
        :description="`Câu đã có ${question.usedCount} lượt làm. Sửa sẽ tạo phiên bản mới; bài đã làm vẫn giữ đúng đề cũ.`"
      />
      <div class="grid-3">
        <hui-select label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="onSubject($event.detail.value)" />
        <hui-select label="Dạng câu" :options="kindOptions" :value="form.kind" @change="setKind($event.detail.value)" />
        <hui-select label="Mức độ" :options="levelOptions" :value="form.level" @change="form.level = $event.detail.value" />
      </div>
      <div class="grid-3">
        <hui-select label="Lớp (không bắt buộc)" allow-clear :options="gradeOptions" :value="form.gradeId" @change="form.gradeId = $event.detail.value || null" />
        <hui-select label="Thư mục" allow-clear :options="folderOptions" :value="form.folderId" @change="form.folderId = $event.detail.value || null" />
        <hui-select label="Danh mục" mode="multiple" :options="tagOptions" :value="form.tagIds" :error="fields.tagIds" @change="form.tagIds = $event.detail.value" />
      </div>

      <MarkdownEditor v-model="form.content" label="Nội dung câu hỏi" :rows="5" :error="fields.content" compact />

      <section v-if="form.kind !== 'short'" class="stack">
        <div class="row between">
          <b>{{ form.kind === 'true_false' ? 'Các ý (đánh dấu ý Đúng)' : 'Phương án (đánh dấu đáp án đúng)' }}</b>
          <hui-button size="sm" variant="outline" :disabled="form.options.length >= maxOptions" @hui-click="addOption">Thêm {{ form.kind === 'true_false' ? 'ý' : 'phương án' }}</hui-button>
        </div>
        <div v-for="(o, i) in form.options" :key="o.key" class="opt-row">
          <label class="opt-correct">
            <input
              :type="form.kind === 'single' ? 'radio' : 'checkbox'"
              name="correct"
              :checked="o.correct"
              @change="setCorrect(i, $event.target.checked)"
            />
            {{ form.kind === 'true_false' ? `${statementLetter(i)}) Đúng` : `${letter(i)}. Đúng` }}
          </label>
          <input class="native-input grow" :value="o.content" placeholder="Nội dung (gõ được $công thức$)" @input="o.content = $event.target.value" />
          <hui-markdown v-if="o.content.includes('$')" class="opt-preview" inline :value="o.content" />
          <hui-button size="sm" variant="text" :disabled="form.options.length <= 2" @hui-click="form.options.splice(i, 1)">Xóa</hui-button>
        </div>
        <p v-if="fields.options" class="field-error">{{ fields.options }}</p>
        <hui-checkbox
          v-if="form.kind !== 'true_false'"
          label="Cho phép trộn thứ tự phương án (bỏ chọn nếu có phương án kiểu &quot;Cả A và B&quot;)"
          :checked="form.shuffleOptions"
          @change="form.shuffleOptions = $event.detail.checked"
        />
      </section>
      <section v-else class="stack">
        <hui-input
          label="Đáp án chấp nhận"
          hint="Nhiều đáp án cách nhau dấu ; (ví dụ: 0,5; 1/2). Số 0,5 và 0.5 được coi là như nhau."
          :value="accepted"
          :error="fields.accepted"
          @input="accepted = $event.detail.value"
        />
      </section>

      <MarkdownEditor v-model="form.explanation" label="Lời giải (không bắt buộc)" :rows="4" compact />
    </form>
  </hui-modal>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, reactive, ref, watch } from 'vue';

import { catalogApi, teachingApi } from '../../api';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { LEVEL_LABELS, QUESTION_KIND_LABELS, letter, statementLetter, toOptions } from '../../utils/teaching';
import MarkdownEditor from './MarkdownEditor.vue';

const props = defineProps({
  open: { type: Boolean, default: false },
  /** QuestionResponse khi sửa; null khi thêm */
  question: { type: Object, default: null },
  defaultSubjectId: { type: String, default: null },
  defaultFolderId: { type: String, default: null },
  folders: { type: Array, default: () => [] },
});
const emit = defineEmits(['update:open', 'saved']);

const { data: master, load: loadMaster } = useMasterData();
const { loading, error, fields, run, reset } = useSubmit();
const tags = ref([]);
const accepted = ref('');
let seq = 0;
const form = reactive(blank());

function blank() {
  return {
    subjectId: null,
    gradeId: null,
    folderId: null,
    kind: 'single',
    level: 'nhan_biet',
    tagIds: [],
    content: '',
    options: [],
    explanation: '',
    shuffleOptions: true,
  };
}

const option = (content = '', correct = false, id = null) => ({ key: ++seq, id, content, correct });
const maxOptions = computed(() => (form.kind === 'true_false' ? 6 : 8));

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ value: g.id, label: g.name })));
const levelOptions = toOptions(LEVEL_LABELS);
const kindOptions = toOptions(QUESTION_KIND_LABELS);
const tagOptions = computed(() => tags.value.map((t) => ({ value: t.id, label: t.name })));
const folderOptions = computed(() => props.folders.map((f) => ({ value: f.id, label: f.path ?? f.name })));

async function loadTags(subjectId) {
  tags.value = subjectId ? await catalogApi.tags(subjectId).catch(() => []) : [];
}

function onSubject(id) {
  form.subjectId = id;
  form.tagIds = [];
  loadTags(id);
}

function setKind(kind) {
  const prev = form.kind;
  form.kind = kind;
  if (kind === 'short') {
    form.options = [];
  } else if (prev === 'short' || form.options.length < 2) {
    form.options = kind === 'true_false' ? [0, 1, 2, 3].map(() => option()) : [0, 1, 2, 3].map(() => option());
  }
  if (kind === 'single') {
    let seen = false;
    for (const o of form.options) {
      if (o.correct && !seen) seen = true;
      else o.correct = false;
    }
  }
}

function setCorrect(i, checked) {
  if (form.kind === 'single') form.options.forEach((o, j) => (o.correct = j === i && checked));
  else form.options[i].correct = checked;
}

function addOption() {
  form.options.push(option());
}

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    reset();
    await loadMaster();
    const q = props.question;
    Object.assign(form, blank());
    if (q) {
      Object.assign(form, {
        subjectId: q.subjectId,
        gradeId: q.gradeId,
        folderId: q.folderId,
        kind: q.kind,
        level: q.level,
        tagIds: [...q.tagIds],
        content: q.content,
        options: q.options.map((o) => option(o.content, o.correct, o.id)),
        explanation: q.explanation,
        shuffleOptions: q.shuffleOptions,
      });
      accepted.value = q.accepted.join('; ');
    } else {
      form.subjectId = props.defaultSubjectId || master.value?.subjects[0]?.id || null;
      form.folderId = props.defaultFolderId;
      form.options = [0, 1, 2, 3].map(() => option());
      accepted.value = '';
    }
    loadTags(form.subjectId);
  },
);

function payload() {
  return {
    subjectId: form.subjectId,
    gradeId: form.gradeId,
    folderId: form.folderId,
    kind: form.kind,
    level: form.level,
    tagIds: form.tagIds,
    content: form.content,
    options: form.kind === 'short' ? [] : form.options.map((o) => ({ id: o.id, content: o.content, correct: o.correct })),
    accepted: form.kind === 'short' ? accepted.value.split(';').map((a) => a.trim()).filter(Boolean) : [],
    explanation: form.explanation,
    shuffleOptions: form.shuffleOptions,
  };
}

function onOk(e) {
  e.preventDefault();
  save();
}

async function save() {
  const res = await run(() =>
    props.question ? teachingApi.updateQuestion(props.question.id, payload()) : teachingApi.createQuestion(payload()),
  );
  if (!res.ok) return;
  const saved = props.question ? res.data.question : res.data;
  if (props.question && res.data.answerChanged && res.data.affectedAttempts > 0) {
    // Đáp án đúng đổi khi đã có bài nộp → hỏi chấm lại.
    if (window.confirm(`Đáp án đúng đã đổi. Chấm lại ${res.data.affectedAttempts} bài đã nộp theo đáp án mới?`)) {
      try {
        const r = await teachingApi.regradeQuestion(saved.id);
        message.success(`Đã chấm lại ${r.regradedAttempts} bài, ${r.changedAttempts} bài thay đổi điểm.`);
      } catch (err) {
        message.error(err.message);
      }
    }
  }
  emit('saved', saved);
  close();
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.opt-row {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.opt-row .grow {
  flex: 1 1 260px;
}
.opt-correct {
  display: inline-flex;
  gap: 4px;
  align-items: center;
  min-width: 92px;
  font-size: 0.875rem;
}
.opt-preview {
  flex-basis: 100%;
  padding-left: 100px;
}
</style>

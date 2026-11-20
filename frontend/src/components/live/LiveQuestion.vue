<template>
  <section class="stack lq">
    <div class="row between">
      <b class="small">Câu hỏi từ ngân hàng</b>
      <hui-button v-if="isTeacher && !question" size="sm" variant="outline" @hui-click="pickerOpen = true">Chọn câu…</hui-button>
    </div>

    <template v-if="question">
      <QuestionView :item="item" :number="1" bare :readonly="!canAnswer" @answer="onAnswer" />

      <!-- Học sinh -->
      <template v-if="!isTeacher && !result">
        <div v-if="canAnswer" class="row">
          <hui-button size="sm" :disabled="!answered" @hui-click="submit">{{ sent ? 'Gửi lại' : 'Gửi câu trả lời' }}</hui-button>
          <span v-if="sent" class="muted small">Đã gửi — sửa rồi gửi lại được tới khi giáo viên chốt.</span>
        </div>
        <p v-else class="muted small">Chỉ học sinh trả lời câu hỏi.</p>
      </template>
      <p v-if="result && myResult !== null" class="small verdict" :class="myResult ? 'ok' : 'bad'">
        {{ myResult ? '✓ Bạn trả lời đúng.' : '✗ Bạn trả lời chưa đúng.' }}
      </p>

      <!-- Giáo viên -->
      <template v-if="isTeacher">
        <p class="small" style="margin: 0">
          <b>{{ answers.length }}</b> em đã trả lời<template v-if="result"> · <b>{{ result.rightCount }}</b> em đúng</template>
        </p>
        <div v-if="answers.length" class="names small">
          <span v-for="a in answers" :key="a.identity" class="name" :class="result ? (result.perStudent[a.identity] ? 'ok' : 'bad') : ''">{{ a.name }}</span>
        </div>
        <div class="row">
          <hui-button v-if="!result" size="sm" :loading="closing" @hui-click="emit('close')">Chốt & chấm</hui-button>
          <hui-button v-else size="sm" variant="outline" @hui-click="emit('clear')">Câu khác</hui-button>
        </div>
      </template>
    </template>
    <p v-else-if="!isTeacher" class="muted small">Giáo viên chưa hỏi câu nào.</p>

    <QuestionPickerModal
      v-if="isTeacher"
      v-model:open="pickerOpen"
      :subjectId="subjectId"
      okText="Hỏi cả lớp"
      single
      :kinds="['single', 'multiple', 'true_false', 'short']"
      @picked="(qs) => qs[0] && emit('start', qs[0])"
    />
  </section>
</template>

<script setup>
/**
 * Câu hỏi lấy từ ngân hàng hỏi nhanh cả lớp trong phòng live (M45): giáo viên chọn câu (một / nhiều đáp án, đúng
 * sai, trả lời ngắn), học sinh trả lời bằng `QuestionView`, giáo viên chốt → chấm (trả lời ngắn chấm ở server).
 */
import { computed, ref, watch } from 'vue';

import { isAnswered } from '../../utils/teaching';
import QuestionPickerModal from '../teaching/QuestionPickerModal.vue';
import QuestionView from '../teaching/QuestionView.vue';

const props = defineProps({
  isTeacher: { type: Boolean, default: false },
  /** Học sinh được trả lời (khách / giáo viên thì không) */
  canAnswer: { type: Boolean, default: false },
  /** { id, qkind, content, options: [{ id, content }], expression } — không có đáp án */
  question: { type: Object, default: null },
  /** Kết quả khi chốt: { rightCount, voters, correct: [optionId], accepted, explanation, perStudent } */
  result: { type: Object, default: null },
  /** Giáo viên: [{ identity, name, answer }] */
  answers: { type: Array, default: () => [] },
  myIdentity: { type: String, default: '' },
  subjectId: { type: String, default: '' },
  closing: { type: Boolean, default: false },
});
const emit = defineEmits(['start', 'submit', 'close', 'clear']);

const pickerOpen = ref(false);
const draft = ref(null);
const sent = ref(false);

watch(
  () => props.question?.id,
  () => {
    draft.value = null;
    sent.value = false;
  },
);

const item = computed(() => {
  const q = props.question;
  const r = props.result;
  return {
    position: 0,
    kind: q.qkind,
    content: q.content,
    points: 1,
    options: q.options,
    answer: draft.value,
    checked: false,
    manual: false,
    files: [],
    grading: null,
    expression: Boolean(q.expression),
    result: r
      ? {
          isCorrect: myResult.value ?? false,
          score: myResult.value ? 1 : 0,
          correct: r.correct ?? [],
          accepted: r.accepted ?? [],
          explanation: r.explanation ?? '',
          rubric: [],
        }
      : null,
  };
});
const answered = computed(() => isAnswered({ kind: props.question?.qkind, options: props.question?.options ?? [], answer: draft.value }));
const myResult = computed(() => {
  const r = props.result;
  if (!r || props.isTeacher || !(props.myIdentity in (r.perStudent ?? {}))) return null;
  return Boolean(r.perStudent[props.myIdentity]);
});

function onAnswer({ answer }) {
  draft.value = { ...(draft.value ?? {}), ...answer };
}

function submit() {
  sent.value = true;
  emit('submit', draft.value);
}
</script>

<style scoped>
.lq {
  padding-top: 8px;
  border-top: 1px solid var(--hui-border-secondary, #eceef1);
}
.names {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.name {
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--hui-bg-secondary, #f5f6f8);
}
.name.ok,
.verdict.ok {
  color: var(--hui-success, #389e0d);
}
.name.bad,
.verdict.bad {
  color: var(--hui-danger, #d4380d);
}
</style>

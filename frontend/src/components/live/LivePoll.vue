<template>
  <div class="stack poll">
    <!-- Đang có câu hỏi -->
    <section v-if="poll" class="card stack">
      <div class="row between">
        <hui-tag size="sm" :color="poll.kind === 'quiz' ? 'warning' : 'info'">{{ poll.kind === 'quiz' ? 'Quiz nhanh' : 'Bình chọn' }}</hui-tag>
        <span class="muted small">{{ results?.voters ?? 0 }} người đã chọn</span>
      </div>
      <hui-markdown class="question" :value="poll.question" />
      <div class="options">
        <button
          v-for="(o, i) in poll.options"
          :key="i"
          type="button"
          class="option"
          :class="{ mine: myVote === i, correct: showAnswer && results.correct?.includes(i) }"
          :disabled="!canVote"
          @click="$emit('vote', i)"
        >
          <span class="letter">{{ String.fromCharCode(65 + i) }}</span>
          <span class="text">{{ o }}</span>
          <span v-if="showCounts" class="pct">{{ percent(results.counts[i], results.voters) }}%</span>
          <span v-if="showCounts" class="bar" :style="{ width: `${percent(results.counts[i], results.voters)}%` }" />
        </button>
      </div>
      <p v-if="!isTeacher && !results?.closed" class="muted small" style="margin: 0">
        {{ myVote == null ? 'Chọn một đáp án.' : 'Đã chọn — đổi được tới khi giáo viên đóng.' }}
      </p>
      <p v-if="results?.closed && poll.kind === 'quiz' && !isTeacher" class="small" style="margin: 0">
        <b :class="results.correct?.includes(myVote) ? 'ok' : 'field-error'">
          {{ myVote == null ? 'Bạn chưa chọn.' : results.correct?.includes(myVote) ? 'Chính xác!' : 'Chưa đúng.' }}
        </b>
      </p>
      <div v-if="isTeacher" class="row">
        <hui-button v-if="!results?.closed" size="sm" @hui-click="$emit('close')">Đóng &amp; công bố kết quả</hui-button>
        <hui-button v-else size="sm" variant="outline" @hui-click="$emit('clear')">Câu hỏi mới</hui-button>
      </div>
    </section>

    <!-- Giáo viên soạn câu hỏi -->
    <form v-else-if="isTeacher" class="card stack" @submit.prevent="start">
      <div class="row">
        <label class="small"><input v-model="kind" type="radio" value="poll" /> Bình chọn</label>
        <label class="small"><input v-model="kind" type="radio" value="quiz" /> Quiz (có đáp án đúng)</label>
      </div>
      <textarea v-model="question" class="native-input" rows="2" maxlength="500" placeholder="Câu hỏi (viết công thức trong $…$)" />
      <div v-for="(o, i) in options" :key="i" class="row opt-row">
        <input v-if="kind === 'quiz'" v-model="correct" type="radio" :value="i" :title="`Đáp án đúng: ${String.fromCharCode(65 + i)}`" />
        <b>{{ String.fromCharCode(65 + i) }}</b>
        <input v-model="options[i]" class="native-input grow" maxlength="200" />
        <button v-if="options.length > 2" type="button" class="x" title="Bỏ phương án" @click="removeOption(i)">✕</button>
      </div>
      <div class="row">
        <hui-button v-if="options.length < 6" size="sm" variant="text" @hui-click="options.push('')">+ Phương án</hui-button>
        <hui-button size="sm" type="submit" :disabled="!ready">Mở cho cả lớp</hui-button>
      </div>
    </form>
    <p v-else class="muted small">Chưa có câu hỏi nào. Giáo viên mở bình chọn / quiz thì hiện ở đây.</p>

    <!-- Đã hỏi trong buổi -->
    <section v-if="history.length" class="stack">
      <b class="small">Đã hỏi trong buổi</b>
      <div v-for="h in history" :key="h.id" class="small hist">
        <hui-markdown inline :value="h.question" />
        <span class="muted"> — {{ h.options.map((o, i) => `${String.fromCharCode(65 + i)}: ${h.counts[i]}`).join(' · ') }}</span>
      </div>
    </section>
  </div>
</template>

<script setup>
/** Bình chọn / quiz nhanh trong buổi học (giáo viên mở, học sinh chọn, kết quả hiện ngay). */
import { computed, ref } from 'vue';

import { percent } from '../../utils/live';

const props = defineProps({
  isTeacher: { type: Boolean, default: false },
  /** { id, kind, question, options } */
  poll: { type: Object, default: null },
  /** { counts, voters, correct, closed } */
  results: { type: Object, default: null },
  myVote: { type: Number, default: null },
  /** Khách chỉ xem */
  canVoteRole: { type: Boolean, default: true },
  history: { type: Array, default: () => [] },
});
const emit = defineEmits(['start', 'vote', 'close', 'clear']);

const kind = ref('poll');
const question = ref('');
const options = ref(['', '']);
const correct = ref(0);

const ready = computed(() => question.value.trim() && options.value.filter((o) => o.trim()).length >= 2);
const canVote = computed(() => !props.isTeacher && props.canVoteRole && !props.results?.closed);
/** Giáo viên luôn thấy số phiếu; học sinh thấy khi đã đóng (quiz) hoặc ngay (bình chọn). */
const showCounts = computed(() => !!props.results && (props.isTeacher || props.results.closed || props.poll?.kind === 'poll'));
const showAnswer = computed(() => !!props.results?.closed && props.poll?.kind === 'quiz');

function removeOption(i) {
  options.value.splice(i, 1);
  if (correct.value >= options.value.length) correct.value = 0;
}

function start() {
  if (!ready.value) return;
  const list = options.value.map((o) => o.trim());
  const kept = list.map((o, i) => [o, i]).filter(([o]) => o);
  const correctIndex = kept.findIndex(([, i]) => i === correct.value);
  emit('start', {
    kind: kind.value,
    question: question.value.trim(),
    options: kept.map(([o]) => o),
    correct: kind.value === 'quiz' ? [Math.max(0, correctIndex)] : [],
  });
  question.value = '';
  options.value = ['', ''];
  correct.value = 0;
}
</script>

<style scoped>
.card {
  padding: 10px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: 8px;
}
.options {
  display: grid;
  gap: 6px;
}
.option {
  position: relative;
  display: flex;
  gap: 8px;
  align-items: center;
  overflow: hidden;
  padding: 8px 10px;
  border: 1px solid var(--hui-border, #d9d9d9);
  border-radius: 8px;
  background: var(--hui-bg-container, #fff);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.option:disabled {
  cursor: default;
}
.option.mine {
  border-color: var(--hui-primary, #0864ad);
  box-shadow: 0 0 0 1px var(--hui-primary, #0864ad);
}
.option.correct {
  border-color: var(--hui-success, #16a34a);
  background: color-mix(in srgb, var(--hui-success, #16a34a) 10%, transparent);
}
.letter {
  font-weight: 700;
}
.text {
  flex: 1;
  z-index: 1;
}
.pct {
  z-index: 1;
  font-weight: 600;
}
.bar {
  position: absolute;
  inset: 0 auto 0 0;
  background: color-mix(in srgb, var(--hui-primary, #0864ad) 14%, transparent);
}
.opt-row {
  flex-wrap: nowrap;
}
.grow {
  flex: 1;
}
.x {
  border: 0;
  background: none;
  cursor: pointer;
  color: var(--hui-text-secondary, #6b7280);
}
.ok {
  color: var(--hui-success, #16a34a);
}
textarea.native-input {
  font: inherit;
  resize: vertical;
}
.hist {
  padding: 4px 0;
  border-bottom: 1px dashed var(--hui-border-secondary, #eceef1);
}
</style>

<template>
  <article class="q" :class="resultClass">
    <header v-if="!bare" class="q__head">
      <strong>Câu {{ number }}</strong>
      <span class="muted small">{{ QUESTION_KIND_LABELS[item.kind] }} · {{ formatScore(item.points) }} điểm</span>
      <hui-tag v-if="item.result" size="sm" :color="item.result.isCorrect ? 'success' : partial ? 'warning' : 'danger'">
        {{ item.result.isCorrect ? 'Đúng' : partial ? 'Đúng một phần' : 'Sai' }} · {{ formatScore(item.result.score) }} điểm
      </hui-tag>
    </header>
    <hui-markdown class="q__content" :value="item.content" />

    <!-- Một / nhiều đáp án -->
    <ul v-if="item.kind === 'single' || item.kind === 'multiple'" class="q__options">
      <li v-for="(o, i) in item.options" :key="o.id" :class="optionClass(o.id)">
        <label>
          <input
            :type="item.kind === 'single' ? 'radio' : 'checkbox'"
            :name="`q-${item.position}`"
            :checked="selected.includes(o.id)"
            :disabled="locked"
            @change="toggle(o.id, $event.target.checked)"
          />
          <b class="q__letter">{{ letter(i) }}.</b>
          <hui-markdown inline :value="o.content" />
        </label>
      </li>
    </ul>

    <!-- Đúng / sai từng ý -->
    <table v-else-if="item.kind === 'true_false'" class="q__tf">
      <thead>
        <tr>
          <th>Ý</th>
          <th>Đúng</th>
          <th>Sai</th>
          <th v-if="item.result" />
        </tr>
      </thead>
      <tbody>
        <tr v-for="(o, i) in item.options" :key="o.id">
          <td>
            <b>{{ statementLetter(i) }})</b>
            <hui-markdown inline :value="o.content" />
          </td>
          <td class="center">
            <input type="radio" :name="`q-${item.position}-${o.id}`" :checked="truth[o.id] === true" :disabled="locked" @change="mark(o.id, true)" />
          </td>
          <td class="center">
            <input type="radio" :name="`q-${item.position}-${o.id}`" :checked="truth[o.id] === false" :disabled="locked" @change="mark(o.id, false)" />
          </td>
          <td v-if="item.result" class="small" :class="truth[o.id] === undefined ? '' : truth[o.id] === expectedTruth(o.id) ? 'ok' : 'bad'">
            Đáp án: {{ expectedTruth(o.id) ? 'Đúng' : 'Sai' }}
          </td>
        </tr>
      </tbody>
    </table>

    <!-- Trả lời ngắn -->
    <div v-else-if="item.kind === 'short'" class="q__short">
      <input
        class="native-input"
        type="text"
        maxlength="500"
        placeholder="Nhập câu trả lời (số thập phân dùng dấu phẩy hoặc chấm)"
        :value="item.answer?.text ?? ''"
        :disabled="locked"
        @input="emitAnswer({ text: $event.target.value }, true)"
        @change="emitAnswer({ text: $event.target.value })"
      />
      <p v-if="item.result" class="small">
        Đáp án chấp nhận: <b>{{ item.result.accepted.join(' · ') }}</b>
      </p>
    </div>

    <div v-if="item.result?.explanation" class="q__explain">
      <span class="muted small">Lời giải</span>
      <hui-markdown :value="item.result.explanation" />
    </div>
    <slot name="actions" />
  </article>
</template>

<script setup>
/**
 * Một câu hỏi trong lượt làm bài (AttemptView.items[]): làm bài hoặc xem lại (khi có `item.result`).
 * Phát `answer` với câu trả lời mới: { selected } | { truth } | { text }. `typing = true` khi đang gõ (lưu trễ).
 */
import { computed } from 'vue';

import { QUESTION_KIND_LABELS, formatScore, letter, statementLetter } from '../../utils/teaching';

const props = defineProps({
  item: { type: Object, required: true },
  number: { type: Number, required: true },
  /** Không cho sửa (đã nộp, đã kiểm tra, giáo viên xem) */
  readonly: { type: Boolean, default: false },
  /** Ẩn dòng "Câu n · điểm" (giáo viên xem đáp án trong ngân hàng câu hỏi) */
  bare: { type: Boolean, default: false },
});
const emit = defineEmits(['answer']);

const locked = computed(() => props.readonly || props.item.checked || Boolean(props.item.result));
const selected = computed(() => props.item.answer?.selected ?? []);
const truth = computed(() => props.item.answer?.truth ?? {});
const partial = computed(() => props.item.result && !props.item.result.isCorrect && props.item.result.score > 0);
const resultClass = computed(() => {
  if (!props.item.result) return '';
  if (props.item.result.isCorrect) return 'is-correct';
  return partial.value ? 'is-partial' : 'is-wrong';
});

function emitAnswer(answer, typing = false) {
  emit('answer', { position: props.item.position, answer, typing });
}

function toggle(id, checked) {
  if (props.item.kind === 'single') return emitAnswer({ selected: checked ? [id] : [] });
  const next = new Set(selected.value);
  if (checked) next.add(id);
  else next.delete(id);
  emitAnswer({ selected: [...next] });
}

function mark(id, value) {
  emitAnswer({ truth: { ...truth.value, [id]: value } });
}

const expectedTruth = (id) => props.item.result?.correct.includes(id) ?? false;

function optionClass(id) {
  if (!props.item.result) return '';
  const isCorrect = props.item.result.correct.includes(id);
  const chosen = selected.value.includes(id);
  if (isCorrect) return 'opt-correct';
  if (chosen) return 'opt-wrong';
  return '';
}
</script>

<style scoped>
.q {
  display: grid;
  gap: 10px;
  padding: 16px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-lg, 12px);
  background: var(--hui-bg-container, #fff);
}
.q.is-correct {
  border-color: color-mix(in srgb, var(--hui-success, #52c41a) 55%, transparent);
}
.q.is-wrong {
  border-color: color-mix(in srgb, var(--hui-danger, #ff4d4f) 55%, transparent);
}
.q.is-partial {
  border-color: color-mix(in srgb, var(--hui-warning, #faad14) 60%, transparent);
}
.q__head {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: center;
}
.q__options {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 6px;
}
.q__options li {
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.q__options label {
  display: flex;
  gap: 8px;
  align-items: baseline;
  padding: 8px 10px;
  cursor: pointer;
}
.q__options li.opt-correct {
  background: color-mix(in srgb, var(--hui-success, #52c41a) 12%, transparent);
  border-color: color-mix(in srgb, var(--hui-success, #52c41a) 50%, transparent);
}
.q__options li.opt-wrong {
  background: color-mix(in srgb, var(--hui-danger, #ff4d4f) 10%, transparent);
  border-color: color-mix(in srgb, var(--hui-danger, #ff4d4f) 45%, transparent);
}
.q__letter {
  min-width: 1.4em;
}
.q__tf {
  border-collapse: collapse;
  width: 100%;
}
.q__tf th,
.q__tf td {
  border: 1px solid var(--hui-border-secondary, #eceef1);
  padding: 6px 8px;
  text-align: left;
}
.q__tf .center {
  text-align: center;
  width: 64px;
}
.ok {
  color: var(--hui-success, #389e0d);
}
.bad {
  color: var(--hui-danger, #d4380d);
}
.q__short .native-input {
  width: min(100%, 420px);
}
.q__explain {
  border-top: 1px dashed var(--hui-border, #d9d9d9);
  padding-top: 8px;
}
</style>

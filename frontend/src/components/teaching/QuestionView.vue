<template>
  <article class="q" :class="resultClass">
    <header v-if="!bare" class="q__head">
      <strong>Câu {{ number }}</strong>
      <span class="muted small">{{ QUESTION_KIND_LABELS[item.kind] }} · {{ formatScore(item.points) }} điểm</span>
      <hui-tag v-if="item.manual && item.grading" size="sm" :color="item.grading.pending ? 'warning' : 'info'">
        {{ item.grading.pending ? 'Chờ giáo viên chấm' : `${formatScore(item.grading.score)} / ${formatScore(item.points)} điểm` }}
      </hui-tag>
      <hui-tag v-else-if="item.result && !item.manual" size="sm" :color="item.result.isCorrect ? 'success' : partial ? 'warning' : 'danger'">
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
        <template v-if="item.result.matchMode === 'expression'"> · biểu thức tương đương đều đúng</template>
        <template v-if="item.result.tolerance"> · sai số ±{{ formatScore(item.result.tolerance) }}</template>
      </p>
    </div>

    <!-- Tự luận: gõ bài và / hoặc chụp ảnh bài làm -->
    <div v-else-if="item.kind === 'essay'" class="q__essay">
      <template v-if="!locked">
        <textarea
          class="native-input"
          rows="5"
          maxlength="10000"
          placeholder="Gõ bài làm (gõ được $công thức$) — hoặc làm ra giấy rồi chụp ảnh bên dưới"
          :value="item.answer?.text ?? ''"
          @input="emitAnswer({ text: $event.target.value }, true)"
          @change="emitAnswer({ text: $event.target.value })"
        />
        <div class="row">
          <hui-button size="sm" variant="outline" :disabled="files.length >= 10" @hui-click="camera?.click()">📷 Chụp ảnh bài làm</hui-button>
          <hui-button size="sm" variant="outline" :disabled="files.length >= 10" @hui-click="picker?.click()">Chọn ảnh / PDF</hui-button>
          <span class="muted small">Tối đa 10 file · ảnh tự thu nhỏ trước khi gửi</span>
          <input ref="picker" type="file" multiple accept="image/png,image/jpeg,image/gif,image/webp,application/pdf" hidden @change="onFiles" />
          <input ref="camera" type="file" accept="image/*" capture="environment" hidden @change="onFiles" />
        </div>
      </template>
      <hui-markdown v-else-if="item.answer?.text" class="q__essay-text" :value="item.answer.text" />
      <p v-else-if="!files.length" class="muted small">Không có bài làm.</p>
      <FileThumbs v-if="files.length" :files="files" :removable="!locked" @remove="removeFile" />

      <div v-if="item.grading && !item.grading.pending" class="q__grading">
        <span class="muted small">Giáo viên chấm</span>
        <ul v-if="item.result?.rubric?.length" class="q__rubric">
          <li v-for="s in item.result.rubric" :key="s.id">
            <hui-markdown inline :value="s.content" />
            <b>{{ formatScore(item.grading.marks[s.id] ?? 0) }} / {{ formatScore(s.points) }}</b>
          </li>
        </ul>
        <hui-markdown v-if="item.grading.feedback" :value="item.grading.feedback" />
      </div>
      <div v-else-if="item.result?.rubric?.length" class="q__grading">
        <span class="muted small">Hướng dẫn chấm</span>
        <ul class="q__rubric">
          <li v-for="s in item.result.rubric" :key="s.id">
            <hui-markdown inline :value="s.content" />
            <b>{{ formatScore(s.points) }} đ</b>
          </li>
        </ul>
      </div>
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
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, ref } from 'vue';

import { QUESTION_KIND_LABELS, formatScore, letter, statementLetter } from '../../utils/teaching';
import { uploadFile } from '../../utils/upload';
import FileThumbs from '../learning/FileThumbs.vue';

const props = defineProps({
  item: { type: Object, required: true },
  number: { type: Number, required: true },
  /** Không cho sửa (đã nộp, đã kiểm tra, giáo viên xem) */
  readonly: { type: Boolean, default: false },
  /** Ẩn dòng "Câu n · điểm" (giáo viên xem đáp án trong ngân hàng câu hỏi) */
  bare: { type: Boolean, default: false },
});
const emit = defineEmits(['answer']);

const locked = computed(() => props.readonly || props.item.checked || Boolean(props.item.result) || Boolean(props.item.grading));
const selected = computed(() => props.item.answer?.selected ?? []);
const truth = computed(() => props.item.answer?.truth ?? {});
const partial = computed(() => props.item.result && !props.item.result.isCorrect && props.item.result.score > 0);
const resultClass = computed(() => {
  if (!props.item.result || props.item.manual) return '';
  if (props.item.result.isCorrect) return 'is-correct';
  return partial.value ? 'is-partial' : 'is-wrong';
});

/** Tự luận: ảnh bài làm đã lưu (`item.files`) + ảnh đang tải lên. */
const picker = ref(null);
const camera = ref(null);
const uploading = ref([]);
const files = computed(() => [...(props.item.files ?? []), ...uploading.value]);

/**
 * `fileViews`: danh sách `{ file, annotated }` mới (để trang làm bài hiện ngay ảnh vừa tải lên).
 */
function emitAnswer(answer, typing = false, fileViews = null) {
  emit('answer', { position: props.item.position, answer, typing, fileViews });
}

async function onFiles(e) {
  const picked = [...e.target.files];
  e.target.value = '';
  const room = 10 - files.value.length;
  if (picked.length > room) message.warning(`Mỗi câu nộp tối đa 10 file — chỉ lấy ${Math.max(room, 0)} file đầu.`);
  for (const file of picked.slice(0, Math.max(room, 0))) {
    const id = `tmp-${Math.random()}`;
    uploading.value.push({ id, name: file.name, url: '', isImage: false, progress: 0 });
    // Bản reactive trong mảng (đối tượng gốc không cập nhật giao diện)
    const entry = uploading.value.find((u) => u.id === id);
    try {
      const saved = await uploadFile(file, 'homework', { onProgress: (p) => (entry.progress = p) });
      const next = [...(props.item.files ?? []), { file: saved, annotated: null }];
      emitAnswer({ files: next.map((f) => f.file.id) }, false, next);
    } catch (err) {
      message.error(`${file.name}: ${err.message}`);
    } finally {
      uploading.value = uploading.value.filter((u) => u.id !== id);
    }
  }
}

function removeFile(i) {
  const next = (props.item.files ?? []).filter((_, j) => j !== i);
  emitAnswer({ files: next.map((f) => f.file.id) }, false, next);
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
.q__essay {
  display: grid;
  gap: 8px;
}
.q__essay textarea {
  width: 100%;
  font: inherit;
}
.q__essay-text {
  padding: 8px 10px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-subtle, #f6f7f9);
}
.q__grading {
  display: grid;
  gap: 6px;
  padding: 10px 12px;
  border-radius: var(--hui-radius-md, 8px);
  background: color-mix(in srgb, var(--hui-primary, #1677ff) 6%, transparent);
}
.q__rubric {
  margin: 0;
  padding-left: 18px;
  display: grid;
  gap: 4px;
}
.q__rubric li {
  display: flex;
  gap: 8px;
  justify-content: space-between;
}
</style>

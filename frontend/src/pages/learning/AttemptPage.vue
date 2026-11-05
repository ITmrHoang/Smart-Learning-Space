<template>
  <div class="page attempt-page">
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="view">
      <header class="panel head">
        <div class="stack" style="gap: 4px">
          <RouterLink v-if="backLink" :to="backLink" class="small">← Quay lại</RouterLink>
          <h1 style="margin: 0">{{ view.title }}</h1>
          <span class="muted small">
            {{ EXAM_KIND_LABELS[view.examKind] }} · {{ view.items.length }} câu · {{ formatScore(view.maxScore) }} điểm
            <template v-if="view.isPreview"> · làm thử (giáo viên)</template>
            <template v-if="view.student"> · bài của {{ view.student.fullName }} (@{{ view.student.username }})</template>
          </span>
        </div>
        <div v-if="inProgress" class="row">
          <span class="muted small">Đã làm {{ answeredCount }}/{{ view.items.length }}</span>
          <span v-if="left !== null" class="clock" :class="{ warn: left <= 60 }">⏱ {{ formatClock(left) }}</span>
          <span class="small muted">{{ saveState }}</span>
          <hui-button :loading="submitting" @hui-click="submit">Nộp bài</hui-button>
        </div>
        <div v-else class="score">
          <span class="muted small">Điểm</span>
          <b>{{ formatScore(view.score) }} / {{ formatScore(view.maxScore) }}</b>
          <span class="muted small">Nộp lúc {{ formatDateTime(view.submittedAt) }}</span>
        </div>
      </header>

      <hui-alert
        v-if="!inProgress && !view.reveal"
        severity="info"
        description="Giáo viên chưa cho xem đáp án của đề này. Bạn xem được điểm, chưa xem được đáp án và lời giải."
      />

      <nav class="panel nav" aria-label="Danh sách câu">
        <a
          v-for="(it, i) in view.items"
          :key="it.position"
          :href="`#cau-${i + 1}`"
          :class="navClass(it)"
          @click.prevent="scrollTo(i)"
        >{{ i + 1 }}</a>
      </nav>

      <template v-for="(it, i) in view.items" :key="it.position">
        <h2 v-if="i === 0 || it.sectionPosition !== view.items[i - 1].sectionPosition" class="section-title">{{ it.sectionTitle }}</h2>
        <QuestionView :id="`cau-${i + 1}`" :item="it" :number="i + 1" :readonly="!inProgress || Boolean(view.student)" @answer="onAnswer">
          <template v-if="view.canCheck && !it.checked" #actions>
            <div class="row">
              <hui-button size="sm" variant="outline" :loading="checking === it.position" :disabled="!isAnswered(it)" @hui-click="check(it)">Kiểm tra đáp án</hui-button>
              <span class="muted small">Kiểm tra xong sẽ không sửa được câu này.</span>
            </div>
          </template>
        </QuestionView>
      </template>

      <div v-if="inProgress" class="row between">
        <span class="muted small">Câu trả lời được lưu tự động.</span>
        <hui-button :loading="submitting" @hui-click="submit">Nộp bài</hui-button>
      </div>
    </template>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { useRoute } from 'vue-router';

import { attemptApi } from '../../api';
import QuestionView from '../../components/teaching/QuestionView.vue';
import { useAuthStore } from '../../stores/auth';
import { formatDateTime } from '../../utils/form';
import { EXAM_KIND_LABELS, formatClock, formatScore, isAnswered, secondsLeft } from '../../utils/teaching';

const route = useRoute();
const auth = useAuthStore();
const view = ref(null);
const error = ref('');
const submitting = ref(false);
const checking = ref(null);
const left = ref(null);
const saveState = ref('');
let loadedAt = 0;
let timer = null;
const pendingSaves = new Map();

const inProgress = computed(() => view.value?.status === 'in_progress' && !view.value.student);
const answeredCount = computed(() => view.value?.items.filter(isAnswered).length ?? 0);
const backLink = computed(() => {
  if (!view.value) return null;
  if (view.value.classId) return { name: 'class', params: { id: view.value.classId } };
  if (auth.can('teaching.manage')) return { name: 'teach-exam', params: { id: view.value.examId } };
  return null;
});

function apply(v) {
  view.value = v;
  loadedAt = Date.now();
  tick();
}

function tick() {
  const v = view.value;
  if (!v || v.status !== 'in_progress' || !v.deadlineAt) {
    left.value = null;
    return;
  }
  left.value = secondsLeft(v.deadlineAt, v.now, loadedAt);
  // Hết giờ: server tự nộp ở lần gọi kế tiếp → tải lại để thấy kết quả.
  if (left.value === 0 && !submitting.value) {
    clearInterval(timer);
    timer = null;
    setTimeout(load, 1500);
  }
}

async function load() {
  error.value = '';
  try {
    apply(await attemptApi.get(route.params.id));
    if (!timer && view.value.status === 'in_progress') timer = setInterval(tick, 1000);
  } catch (e) {
    error.value = e.message;
  }
}

/** Lưu ngay (chọn phương án) hoặc trễ 600 ms khi đang gõ câu trả lời ngắn. */
function onAnswer({ position, answer, typing }) {
  if (!inProgress.value) return;
  const item = view.value.items.find((i) => i.position === position);
  item.answer = { ...(item.answer ?? {}), ...answer };
  clearTimeout(pendingSaves.get(position));
  pendingSaves.set(position, setTimeout(() => persist(position, item.answer), typing ? 600 : 0));
}

async function persist(position, answer) {
  pendingSaves.delete(position);
  if (!inProgress.value) return;
  saveState.value = 'Đang lưu…';
  try {
    await attemptApi.save(view.value.id, position, answer);
    saveState.value = 'Đã lưu';
  } catch (e) {
    saveState.value = '';
    if (e.code === 'ATTEMPT_CLOSED') {
      message.warning('Đã hết giờ, bài đã được nộp.');
      load();
    } else {
      message.error(`Không lưu được câu trả lời: ${e.message}`);
    }
  }
}

async function flushSaves() {
  const jobs = [];
  for (const [position, t] of pendingSaves) {
    clearTimeout(t);
    const item = view.value.items.find((i) => i.position === position);
    jobs.push(persist(position, item.answer));
  }
  await Promise.all(jobs);
}

async function check(it) {
  checking.value = it.position;
  try {
    await flushSaves();
    apply(await attemptApi.check(view.value.id, it.position));
  } catch (e) {
    message.error(e.message);
  } finally {
    checking.value = null;
  }
}

async function submit() {
  const missing = view.value.items.length - answeredCount.value;
  const ask = missing > 0 ? `Còn ${missing} câu chưa làm. Vẫn nộp bài?` : 'Nộp bài?';
  if (!window.confirm(ask)) return;
  submitting.value = true;
  try {
    await flushSaves();
    apply(await attemptApi.submit(view.value.id));
    clearInterval(timer);
    timer = null;
    window.scrollTo({ top: 0, behavior: 'smooth' });
  } catch (e) {
    message.error(e.message);
    if (e.code === 'ATTEMPT_CLOSED') load();
  } finally {
    submitting.value = false;
  }
}

function navClass(it) {
  if (it.result) return it.result.isCorrect ? 'ok' : it.result.score > 0 ? 'partial' : 'bad';
  return isAnswered(it) ? 'done' : '';
}

function scrollTo(i) {
  document.getElementById(`cau-${i + 1}`)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
}

watch(() => route.params.id, () => {
  clearInterval(timer);
  timer = null;
  load();
}, { immediate: true });

onBeforeUnmount(() => {
  clearInterval(timer);
  flushSaves();
});
</script>

<style scoped>
.attempt-page {
  max-width: 900px;
}
.head {
  position: sticky;
  top: 8px;
  z-index: 5;
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  justify-content: space-between;
  align-items: center;
  box-shadow: var(--hui-shadow-sm, 0 2px 8px rgb(0 0 0 / 6%));
}
.clock {
  font-variant-numeric: tabular-nums;
  font-weight: 700;
  font-size: 1.1rem;
}
.clock.warn {
  color: var(--hui-danger, #d4380d);
}
.score {
  display: grid;
  justify-items: end;
}
.score b {
  font-size: 1.6rem;
}
.nav {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 12px;
}
.nav a {
  width: 34px;
  height: 34px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  border: 1px solid var(--hui-border, #d9d9d9);
  color: inherit;
  text-decoration: none;
  font-size: 0.875rem;
}
.nav a.done {
  background: color-mix(in srgb, var(--hui-primary, #1677ff) 14%, transparent);
  border-color: var(--hui-primary, #1677ff);
}
.nav a.ok {
  background: color-mix(in srgb, var(--hui-success, #52c41a) 20%, transparent);
}
.nav a.partial {
  background: color-mix(in srgb, var(--hui-warning, #faad14) 25%, transparent);
}
.nav a.bad {
  background: color-mix(in srgb, var(--hui-danger, #ff4d4f) 15%, transparent);
}
.section-title {
  font-size: 1.05rem;
  margin: 8px 0 0;
}
</style>

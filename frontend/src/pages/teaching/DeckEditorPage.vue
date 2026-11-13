<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <RouterLink to="/day-hoc/the-ghi-nho" class="small">← Thẻ ghi nhớ</RouterLink>
        <h1>{{ isNew ? 'Tạo bộ thẻ' : form.title || 'Bộ thẻ' }}</h1>
      </div>
      <div class="row">
        <hui-button v-if="!isNew" variant="outline" @hui-click="pickerOpen = true">Thêm từ ngân hàng câu hỏi</hui-button>
        <hui-button :loading="loading" @hui-click="save">Lưu</hui-button>
      </div>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />
    <hui-alert v-for="(msg, key) in otherErrors" :key="key" severity="error" :description="msg" />

    <section class="panel form-grid">
      <div class="grid-2">
        <hui-input v-model="form.title" label="Tên bộ thẻ" placeholder="Ví dụ: Công thức lũy thừa" :error="fields.title" required />
        <hui-select label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="form.subjectId = $event.detail.value" />
      </div>
      <hui-textarea label="Mô tả (không bắt buộc)" :rows="2" :value="form.description" @input="form.description = $event.detail.value" />
    </section>

    <section class="panel stack">
      <div class="row between">
        <h2 style="margin: 0">{{ form.cards.length }} thẻ</h2>
        <span class="muted small">Mặt trước / mặt sau viết Markdown, công thức trong $…$. Sửa thẻ không làm mất lịch ôn của học sinh.</span>
      </div>
      <p v-if="!form.cards.length" class="muted">Chưa có thẻ. Bấm "+ Thẻ" hoặc thêm từ ngân hàng câu hỏi.</p>
      <div v-for="(c, i) in form.cards" :key="c.key" class="card-row">
        <span class="num">{{ i + 1 }}</span>
        <div class="faces">
          <label class="face">
            <span class="small muted">Mặt trước</span>
            <textarea v-model="c.front" class="native-input" rows="3" placeholder="Câu hỏi / khái niệm" />
          </label>
          <label class="face">
            <span class="small muted">Mặt sau</span>
            <textarea v-model="c.back" class="native-input" rows="3" placeholder="Đáp án / giải thích" />
          </label>
        </div>
        <span class="tools">
          <button title="Xem thử" @click="previewIndex = i">👁</button>
          <button :disabled="i === 0" title="Lên" @click="move(i, -1)">↑</button>
          <button :disabled="i === form.cards.length - 1" title="Xuống" @click="move(i, 1)">↓</button>
          <button title="Xóa thẻ" @click="form.cards.splice(i, 1)">✕</button>
        </span>
      </div>
      <div class="row">
        <hui-button size="sm" variant="outline" @hui-click="addCard">+ Thẻ</hui-button>
        <hui-button v-if="!isNew" size="sm" variant="text" @hui-click="archive">Ẩn bộ thẻ</hui-button>
      </div>
    </section>

    <section v-if="previewCard" class="panel stack">
      <div class="row between">
        <h2 style="margin: 0">Xem thử thẻ {{ previewIndex + 1 }}</h2>
        <hui-button size="sm" variant="text" @hui-click="previewIndex = -1">Đóng</hui-button>
      </div>
      <hui-flashcard :front="previewCard.front" :back="previewCard.back" no-rating style="max-width: 560px" />
    </section>

    <QuestionPickerModal v-model:open="pickerOpen" :subjectId="form.subjectId" @picked="addQuestions" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import QuestionPickerModal from '../../components/teaching/QuestionPickerModal.vue';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';

const route = useRoute();
const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const { loading, error, fields, run } = useSubmit();
const form = reactive({ title: '', subjectId: null, description: '', cards: [] });
const pickerOpen = ref(false);
const previewIndex = ref(-1);
let seq = 0;

const isNew = computed(() => route.params.id === 'moi');
const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const previewCard = computed(() => form.cards[previewIndex.value] ?? null);
const otherErrors = computed(() => Object.fromEntries(Object.entries(fields).filter(([k]) => !['title', 'subjectId'].includes(k))));

function apply(d) {
  Object.assign(form, {
    title: d.title,
    subjectId: d.subjectId,
    description: d.description ?? '',
    cards: d.cards.map((c) => ({ ...c, key: ++seq })),
  });
}

function addCard() {
  form.cards.push({ id: null, front: '', back: '', key: ++seq });
}

function move(i, d) {
  const [x] = form.cards.splice(i, 1);
  form.cards.splice(i + d, 0, x);
}

function payload() {
  return {
    title: form.title.trim(),
    subjectId: form.subjectId,
    description: form.description,
    cards: form.cards.filter((c) => c.front.trim() || c.back.trim()).map((c) => ({ id: c.id, front: c.front, back: c.back })),
  };
}

async function save() {
  const res = await run(() => (isNew.value ? teachingApi.createDeck(payload()) : teachingApi.updateDeck(route.params.id, payload())));
  if (!res.ok) return;
  message.success('Đã lưu bộ thẻ.');
  apply(res.data);
  if (isNew.value) router.replace({ name: 'teach-deck', params: { id: res.data.id } });
}

async function addQuestions(questions) {
  if (!questions.length) return;
  // Lưu thay đổi đang soạn trước, rồi thêm thẻ từ câu hỏi.
  const saved = await run(() => teachingApi.updateDeck(route.params.id, payload()));
  if (!saved.ok) return;
  const res = await run(() => teachingApi.deckFromQuestions(route.params.id, questions.map((q) => q.id)));
  if (res.ok) {
    apply(res.data);
    message.success(`Đã thêm ${questions.length} thẻ từ câu hỏi.`);
  }
}

async function archive() {
  if (!window.confirm('Ẩn bộ thẻ này? Lộ trình đang dùng vẫn giữ, nhưng không thêm được vào lộ trình mới.')) return;
  try {
    await teachingApi.archiveDeck(route.params.id);
    message.success('Đã ẩn bộ thẻ.');
    router.push('/day-hoc/the-ghi-nho');
  } catch (e) {
    message.error(e.message);
  }
}

onMounted(async () => {
  await loadMaster();
  if (isNew.value) {
    form.subjectId = master.value?.subjects[0]?.id ?? null;
    addCard();
    return;
  }
  try {
    apply(await teachingApi.deck(route.params.id));
  } catch (e) {
    error.value = e.message;
  }
});
</script>

<style scoped>
.card-row {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 10px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.num {
  min-width: 1.5em;
  padding-top: 22px;
  text-align: right;
  color: var(--hui-text-secondary, #6b7280);
}
.faces {
  flex: 1;
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}
.face {
  display: grid;
  gap: 4px;
}
.face textarea {
  width: 100%;
  resize: vertical;
}
.tools {
  display: flex;
  gap: 2px;
  padding-top: 20px;
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
@media (max-width: 700px) {
  .faces {
    grid-template-columns: 1fr;
  }
}
</style>

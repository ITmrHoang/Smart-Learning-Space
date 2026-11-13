<template>
  <div class="page">
    <div class="page-header">
      <h1>Bài tập về nhà</h1>
      <hui-button @hui-click="openEditor(null)">Soạn bài tập</hui-button>
    </div>
    <p class="muted small">
      Bài tập để học sinh làm trên giấy rồi chụp ảnh / gửi PDF. Thêm bài tập vào lộ trình của lớp học bằng nút
      "+ Bài tập về nhà" (đặt hạn nộp ở từng lớp học); xem và chấm bài ở tab "Bài tập" của lớp học.
    </p>
    <div class="toolbar">
      <hui-input class="grow" type="search" label="Tìm" clearable @hui-search="search($event.detail.value)" @clear="search('')" />
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />
    <div v-if="loaded && !items.length" class="panel empty muted">Chưa có bài tập nào.</div>
    <div class="card-list">
      <article v-for="a in items" :key="a.id" class="card">
        <a href="#" class="title" @click.prevent="openEditor(a)"><h3>{{ a.title }}</h3></a>
        <div class="row">
          <hui-tag size="sm">{{ subjectName(a.subjectId) }}</hui-tag>
          <hui-tag size="sm" :color="a.maxScore != null ? 'info' : 'secondary'">
            {{ a.maxScore != null ? `Thang điểm ${formatScore(a.maxScore)}` : 'Chỉ nhận xét' }}
          </hui-tag>
          <hui-tag size="sm" :color="a.submissionCount ? 'success' : 'secondary'">{{ a.submissionCount }} bài nộp</hui-tag>
        </div>
        <p class="muted small" style="margin: 0">Cập nhật {{ formatDateTime(a.updatedAt) }}</p>
      </article>
    </div>

    <hui-modal
      :open="editorOpen"
      :title="editing ? 'Sửa bài tập' : 'Soạn bài tập'"
      size="lg"
      ok-text="Lưu"
      cancel-text="Đóng"
      :confirmLoading="loading"
      @hui-ok="onOk"
      @hui-close="editorOpen = false"
    >
      <form class="form-grid" novalidate @submit.prevent="save">
        <hui-alert v-if="formError" severity="error" :description="formError" />
        <hui-input v-model="form.title" label="Tên bài tập" placeholder="Ví dụ: Bài tập về nhà — Hệ phương trình" :error="fields.title" required />
        <div class="row">
          <hui-select class="grow" label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="form.subjectId = $event.detail.value" />
          <label class="small inline">
            <input v-model="graded" type="checkbox" /> Chấm điểm, thang
            <input v-model.number="form.maxScore" type="number" min="0.25" step="0.25" class="native-input score" :disabled="!graded" />
          </label>
        </div>
        <p v-if="fields.maxScore" class="field-error">{{ fields.maxScore }}</p>
        <MarkdownEditor v-model="form.instructions" label="Đề bài" :rows="8" compact :error="fields.instructions" placeholder="Đề bài, yêu cầu trình bày; công thức viết trong $…$; chèn ảnh hình vẽ nếu cần" />
        <p class="muted small" style="margin: 0">Sửa bài tập thì mọi lớp học đang dùng thấy nội dung mới ngay.</p>
        <hui-button v-if="editing" variant="text" color="danger" @hui-click="archive">Ẩn bài tập này</hui-button>
      </form>
    </hui-modal>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';

import { teachingApi } from '../../api';
import MarkdownEditor from '../../components/teaching/MarkdownEditor.vue';
import { formatDateTime, useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { formatScore } from '../../utils/teaching';

const { data: master, load: loadMaster } = useMasterData();
const { loading, error: formError, fields, run, reset } = useSubmit();
const items = ref([]);
const loaded = ref(false);
const error = ref('');
const editorOpen = ref(false);
const editing = ref(null);
const graded = ref(true);
const form = reactive({ title: '', subjectId: null, instructions: '', maxScore: 10 });

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const subjectName = (id) => master.value?.subjects.find((s) => s.id === id)?.name ?? '';

async function search(q = '') {
  error.value = '';
  try {
    items.value = await teachingApi.assignments({ q: q || undefined });
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
}

async function openEditor(a) {
  reset();
  editing.value = a;
  if (a) {
    const d = await teachingApi.assignment(a.id);
    Object.assign(form, { title: d.title, subjectId: d.subjectId, instructions: d.instructions, maxScore: d.maxScore ?? 10 });
    graded.value = d.maxScore != null;
  } else {
    Object.assign(form, { title: '', subjectId: master.value?.subjects[0]?.id ?? null, instructions: '', maxScore: 10 });
    graded.value = true;
  }
  editorOpen.value = true;
}

async function save() {
  const body = {
    title: form.title.trim(),
    subjectId: form.subjectId,
    instructions: form.instructions,
    maxScore: graded.value ? Number(form.maxScore) : null,
  };
  const res = await run(() =>
    editing.value ? teachingApi.updateAssignment(editing.value.id, body) : teachingApi.createAssignment(body),
  );
  if (res.ok) {
    message.success('Đã lưu bài tập.');
    editorOpen.value = false;
    search();
  }
}

function onOk(e) {
  e.preventDefault();
  save();
}

async function archive() {
  if (!window.confirm('Ẩn bài tập này? Lớp học đang dùng vẫn nộp / chấm được, nhưng không xuất bản lại lộ trình có bài tập này được.')) return;
  try {
    await teachingApi.archiveAssignment(editing.value.id);
    editorOpen.value = false;
    message.success('Đã ẩn bài tập.');
    search();
  } catch (e) {
    message.error(e.message);
  }
}

onMounted(() => {
  loadMaster();
  search();
});
</script>

<style scoped>
.inline {
  display: inline-flex;
  gap: 6px;
  align-items: center;
}
.score {
  width: 80px;
}
</style>

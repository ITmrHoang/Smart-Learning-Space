<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <RouterLink to="/day-hoc/bai-hoc" class="small">← Thư viện bài</RouterLink>
        <h1>{{ isNew ? 'Soạn bài mới' : form.title || 'Bài học' }}</h1>
      </div>
      <div class="row">
        <hui-tag v-if="lesson" :color="statusColor">{{ statusText }}</hui-tag>
        <hui-button variant="outline" :loading="loading" @hui-click="save">Lưu bản nháp</hui-button>
        <hui-button v-if="lesson" :loading="publishing" :disabled="!canPublish" @hui-click="publish">Xuất bản</hui-button>
      </div>
    </div>

    <hui-alert v-if="error" severity="error" :description="error" />
    <section class="panel form-grid">
      <hui-input v-model="form.title" label="Tên bài" :error="fields.title" required />
      <div class="grid-3">
        <hui-select label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="onSubject($event.detail.value)" />
        <hui-select label="Lớp (không bắt buộc)" allow-clear :options="gradeOptions" :value="form.gradeId" @change="form.gradeId = $event.detail.value || null" />
        <hui-select label="Danh mục" mode="multiple" :options="tagOptions" :value="form.tagIds" :error="fields.tagIds" @change="form.tagIds = $event.detail.value" />
      </div>
      <div class="grid-2">
        <hui-input v-model="form.videoUrl" label="Video (link YouTube hoặc link file mp4)" placeholder="https://www.youtube.com/watch?v=…" :error="fields.videoUrl" />
        <hui-input-number label="Thời lượng (phút)" :min="1" :max="600" :value="form.durationMinutes" :error="fields.durationMinutes" @change="form.durationMinutes = $event.detail.value" />
      </div>
      <div v-if="video" class="video">
        <iframe v-if="video.type === 'youtube'" :src="video.src" title="Video bài giảng" allowfullscreen loading="lazy" />
        <video v-else :src="video.src" controls preload="metadata" />
      </div>
      <hui-textarea label="Tóm tắt" :rows="2" :value="form.summary" :error="fields.summary" @input="form.summary = $event.detail.value" />
      <MarkdownEditor v-model="form.content" label="Nội dung bài" :rows="18" :error="fields.content" placeholder="# Tiêu đề&#10;&#10;Nội dung, công thức $\sqrt{2}$…" />
    </section>

    <section v-if="revisions.length" class="panel">
      <h2>Lịch sử xuất bản</h2>
      <ul class="clean-list small">
        <li v-for="r in revisions" :key="r.version">
          <b>v{{ r.version }}</b> · {{ r.title }} · {{ formatDateTime(r.publishedAt) }}
        </li>
      </ul>
    </section>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { catalogApi, teachingApi } from '../../api';
import MarkdownEditor from '../../components/teaching/MarkdownEditor.vue';
import { formatDateTime, useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { videoEmbed } from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const { loading, error, fields, run } = useSubmit();
const isNew = computed(() => route.params.id === 'moi');
const lesson = ref(null);
const revisions = ref([]);
const tags = ref([]);
const publishing = ref(false);
const form = reactive({ subjectId: null, gradeId: null, tagIds: [], title: '', summary: '', content: '', videoUrl: '', durationMinutes: null });

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ value: g.id, label: g.name })));
const tagOptions = computed(() => tags.value.map((t) => ({ value: t.id, label: t.name })));
const video = computed(() => videoEmbed(form.videoUrl?.trim()));
const statusText = computed(() => {
  if (!lesson.value) return '';
  if (lesson.value.publishedVersion == null) return 'Bản nháp — chưa xuất bản';
  return lesson.value.hasChanges ? `Đang dùng v${lesson.value.publishedVersion} · có thay đổi chưa xuất bản` : `Đã xuất bản v${lesson.value.publishedVersion}`;
});
const statusColor = computed(() => (lesson.value?.publishedVersion == null ? 'warning' : lesson.value.hasChanges ? 'info' : 'success'));
const canPublish = computed(() => lesson.value && (lesson.value.publishedVersion == null || lesson.value.hasChanges));

async function onSubject(id) {
  form.subjectId = id;
  form.tagIds = [];
  tags.value = id ? await catalogApi.tags(id).catch(() => []) : [];
}

function apply(l) {
  lesson.value = l;
  Object.assign(form, {
    subjectId: l.subjectId,
    gradeId: l.gradeId,
    tagIds: [...l.tagIds],
    title: l.title,
    summary: l.summary,
    content: l.content,
    videoUrl: l.videoUrl ?? '',
    durationMinutes: l.durationMinutes,
  });
}

function payload() {
  return { ...form, videoUrl: form.videoUrl?.trim() || null, durationMinutes: form.durationMinutes || null };
}

async function save() {
  const res = await run(() => (isNew.value ? teachingApi.createLesson(payload()) : teachingApi.updateLesson(lesson.value.id, payload())));
  if (!res.ok) return false;
  apply(res.data);
  message.success('Đã lưu bản nháp.');
  if (isNew.value) router.replace({ name: 'teach-lesson', params: { id: res.data.id } });
  return true;
}

async function publish() {
  if (!(await save())) return;
  publishing.value = true;
  try {
    apply(await teachingApi.publishLesson(lesson.value.id));
    revisions.value = await teachingApi.lessonRevisions(lesson.value.id);
    message.success('Đã xuất bản. Học sinh các lớp học dùng bài này thấy bản mới.');
  } catch (e) {
    message.error(e.message);
  } finally {
    publishing.value = false;
  }
}

onMounted(async () => {
  await loadMaster();
  if (isNew.value) {
    form.subjectId = master.value?.subjects[0]?.id ?? null;
  } else {
    try {
      apply(await teachingApi.lesson(route.params.id));
      revisions.value = await teachingApi.lessonRevisions(route.params.id);
    } catch (e) {
      error.value = e.message;
      return;
    }
  }
  tags.value = form.subjectId ? await catalogApi.tags(form.subjectId).catch(() => []) : [];
});
</script>

<style scoped>
.video {
  aspect-ratio: 16 / 9;
  max-width: 640px;
  background: #000;
  border-radius: var(--hui-radius-md, 8px);
  overflow: hidden;
}
.video iframe,
.video video {
  width: 100%;
  height: 100%;
  border: 0;
}
</style>

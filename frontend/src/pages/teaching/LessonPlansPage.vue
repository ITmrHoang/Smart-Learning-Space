<template>
  <div class="page">
    <div class="page-header">
      <h1>Giáo án</h1>
      <hui-button @hui-click="createOpen = true">Soạn giáo án mới</hui-button>
    </div>
    <p class="muted small">
      Soạn giáo án theo mẫu CV 5512 (THCS, THPT), CV 2345 (tiểu học) hoặc giáo án dạy thêm; công thức, ảnh, đồ thị, phiếu học tập
      lấy từ ngân hàng câu hỏi. Tải về file Word đúng thể thức hoặc in / lưu PDF.
    </p>

    <section class="panel toolbar">
      <hui-input class="grow" type="search" label="Tìm giáo án" placeholder="Tên bài dạy" clearable @hui-search="setFilter('q', $event.detail.value)" @clear="setFilter('q', '')" />
      <hui-select label="Môn" allow-clear placeholder="Tất cả" :options="subjectOptions" @change="setFilter('subjectId', $event.detail.value)" />
      <hui-select label="Lớp" allow-clear placeholder="Tất cả" :options="gradeOptions" @change="setFilter('gradeId', $event.detail.value)" />
    </section>

    <section class="panel">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-table
        row-key="id"
        pagination="none"
        hover
        :columns="columns"
        :data="rows"
        :loading="loading"
        empty-text="Chưa có giáo án nào. Bấm &quot;Soạn giáo án mới&quot; để bắt đầu."
        @hui-action="onAction"
        @hui-row-click="open($event.detail.row)"
      />
    </section>

    <hui-modal :open="createOpen" title="Soạn giáo án mới" size="md" no-footer @hui-close="createOpen = false">
      <form class="stack" @submit.prevent="create">
        <hui-input v-model="draft.title" label="Tên bài dạy" placeholder="Bài 3. Phương trình bậc hai một ẩn" :error="fields.title" required />
        <div class="grid-2">
          <hui-select label="Môn học" :options="subjectOptions" :value="draft.subjectId" :error="fields.subjectId" @change="draft.subjectId = $event.detail.value" />
          <hui-select label="Lớp (không bắt buộc)" allow-clear :options="gradeOptions" :value="draft.gradeId" @change="draft.gradeId = $event.detail.value || null" />
        </div>
        <fieldset class="templates">
          <legend class="small"><b>Mẫu giáo án</b></legend>
          <label v-for="(t, key) in TEMPLATES" :key="key" class="template" :class="{ active: draft.template === key }">
            <input v-model="draft.template" type="radio" name="template" :value="key" />
            <span>
              <b>{{ t.label }}</b><br />
              <span class="muted small">{{ t.description }}</span>
            </span>
          </label>
        </fieldset>
        <hui-alert v-if="formError" severity="error" :description="formError" />
        <div class="form-actions">
          <hui-button variant="outline" @hui-click="createOpen = false">Hủy</hui-button>
          <hui-button type="submit" :loading="creating">Bắt đầu soạn</hui-button>
        </div>
      </form>
    </hui-modal>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';
import { useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import { useAuthStore } from '../../stores/auth';
import { TEMPLATES, planPayload } from '../../utils/lessonPlan';
import { useMasterData } from '../../utils/masterData';

const router = useRouter();
const auth = useAuthStore();
const { data: master, load: loadMaster } = useMasterData();
const items = ref([]);
const loading = ref(false);
const error = ref('');
const filters = reactive({ q: '', subjectId: '', gradeId: '' });
const createOpen = ref(false);
const creating = ref(false);
const formError = ref('');
const fields = ref({});
const draft = reactive({ title: '', subjectId: null, gradeId: null, template: 'cv5512' });

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ value: g.id, label: g.name })));
const nameOf = (list, id) => list?.find((x) => x.id === id)?.name ?? '';
const rows = computed(() =>
  items.value.map((p) => ({
    ...p,
    subject: nameOf(master.value?.subjects, p.subjectId),
    grade: nameOf(master.value?.grades, p.gradeId),
    templateName: TEMPLATES[p.template]?.label ?? p.template,
  })),
);
const columns = [
  { key: 'title', title: 'Tên bài dạy' },
  { key: 'subject', title: 'Môn', width: '120px' },
  { key: 'grade', title: 'Lớp', width: '90px' },
  { key: 'templateName', title: 'Mẫu', width: '180px' },
  { key: 'blockCount', title: 'Số mục', width: '80px' },
  { key: 'updatedAt', title: 'Sửa lần cuối', type: 'datetime', width: '170px' },
  {
    key: 'actions',
    title: '',
    type: 'actions',
    actions: [
      { key: 'edit', label: 'Soạn' },
      { key: 'copy', label: 'Sao chép' },
      { key: 'archive', label: 'Xóa', danger: true },
    ],
  },
];

async function load() {
  loading.value = true;
  error.value = '';
  try {
    items.value = await teachingApi.lessonPlans({ q: filters.q, subjectId: filters.subjectId, gradeId: filters.gradeId });
  } catch (e) {
    error.value = e.message;
  } finally {
    loading.value = false;
  }
}

function setFilter(key, value) {
  filters[key] = value ?? '';
  load();
}

function open(row) {
  router.push({ name: 'teach-plan', params: { id: row.id } });
}

async function create() {
  formError.value = '';
  fields.value = {};
  creating.value = true;
  try {
    // Thông tin chung lấy từ giáo án gần nhất (trường, tổ, giáo viên thường không đổi)
    const last = items.value[0] ? await teachingApi.lessonPlan(items.value[0].id).catch(() => null) : null;
    const info = { ...(last?.info ?? {}), teacherName: last?.info?.teacherName || auth.user?.fullName || '' };
    for (const k of ['periods', 'preparedOn', 'taughtOn', 'className']) delete info[k];
    const plan = await teachingApi.createLessonPlan(
      planPayload({ ...draft, info, blocks: TEMPLATES[draft.template].blocks(), lessonId: null, attachments: [] }),
    );
    createOpen.value = false;
    router.push({ name: 'teach-plan', params: { id: plan.id } });
  } catch (e) {
    fields.value = Object.fromEntries(Object.entries(e.fields ?? {}).map(([k, v]) => [k, v[0]]));
    formError.value = e.fields ? '' : e.message;
  } finally {
    creating.value = false;
  }
}

async function onAction({ detail: { action, row } }) {
  try {
    if (action === 'edit') return open(row);
    if (action === 'copy') {
      const copy = await teachingApi.copyLessonPlan(row.id);
      message.success('Đã sao chép giáo án.');
      return router.push({ name: 'teach-plan', params: { id: copy.id } });
    }
    if (action === 'archive' && window.confirm(`Xóa giáo án "${row.title}"?`)) {
      await teachingApi.archiveLessonPlan(row.id);
      message.success('Đã xóa giáo án.');
      load();
    }
  } catch (e) {
    message.error(e.message);
  }
}

onMounted(async () => {
  await loadMaster();
  draft.subjectId = master.value?.subjects[0]?.id ?? null;
  load();
});
</script>

<style scoped>
.templates {
  display: grid;
  gap: 8px;
  border: 0;
  margin: 0;
  padding: 0;
}
.template {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  padding: 10px 12px;
  border: 1px solid var(--hui-border-secondary, #e5e7eb);
  border-radius: var(--hui-radius-md, 8px);
  cursor: pointer;
}
.template.active {
  border-color: var(--hui-primary, #1677ff);
  background: color-mix(in srgb, var(--hui-primary, #1677ff) 6%, transparent);
}
.template input {
  margin-top: 4px;
}
</style>

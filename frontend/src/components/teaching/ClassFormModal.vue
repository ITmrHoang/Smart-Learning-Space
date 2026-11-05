<template>
  <hui-modal :open="open" :title="cls ? 'Sửa lớp học' : 'Tạo lớp học'" size="lg" ok-text="Lưu" cancel-text="Hủy" :confirmLoading="loading" @hui-ok="onOk" @hui-close="close">
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input v-model="form.title" label="Tên lớp học" placeholder="Ví dụ: Toán 9 — ôn thi vào 10" :error="fields.title" required />
      <div class="grid-3">
        <hui-select label="Môn học" :options="subjectOptions" :value="form.subjectId" :error="fields.subjectId" @change="onSubject($event.detail.value)" />
        <hui-select label="Lớp" mode="multiple" placeholder="Chọn lớp" :options="gradeOptions" :value="form.gradeIds" :error="fields.gradeIds" @change="form.gradeIds = $event.detail.value" />
        <hui-select label="Danh mục" mode="multiple" :options="tagOptions" :value="form.tagIds" :error="fields.tagIds" @change="form.tagIds = $event.detail.value" />
      </div>
      <div class="grid-3">
        <hui-select label="Hình thức" :options="modeOptions" :value="form.mode" @change="form.mode = $event.detail.value" />
        <hui-select label="Ai thấy lớp học" :options="visibilityOptions" :value="form.visibility" @change="form.visibility = $event.detail.value" />
        <hui-select label="Cách ghi danh" :options="enrollmentOptions" :value="form.enrollment" @change="form.enrollment = $event.detail.value" />
      </div>
      <p class="muted small">
        Công khai: hiện trong trang tìm lớp học. Có link mới thấy: chỉ ai có link. Riêng tư: chỉ học sinh được thêm hoặc có mã mời.
        Mã mời luôn cho vào học ngay, bất kể cách ghi danh.
      </p>
      <MarkdownEditor v-model="form.description" label="Giới thiệu" :rows="6" compact />
    </form>
  </hui-modal>
</template>

<script setup>
import { computed, reactive, ref, watch } from 'vue';

import { catalogApi, teachingApi } from '../../api';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { ENROLLMENT_LABELS, MODE_LABELS, VISIBILITY_LABELS, toOptions } from '../../utils/teaching';
import MarkdownEditor from './MarkdownEditor.vue';

const props = defineProps({
  open: { type: Boolean, default: false },
  /** ClassResponse khi sửa */
  cls: { type: Object, default: null },
});
const emit = defineEmits(['update:open', 'saved']);
const { data: master, load: loadMaster } = useMasterData();
const { loading, error, fields, run, reset } = useSubmit();
const tags = ref([]);
const form = reactive({});

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ value: g.id, label: g.name })));
const tagOptions = computed(() => tags.value.map((t) => ({ value: t.id, label: t.name })));
const modeOptions = toOptions(MODE_LABELS);
const visibilityOptions = toOptions(VISIBILITY_LABELS);
const enrollmentOptions = toOptions(ENROLLMENT_LABELS);

async function onSubject(id) {
  form.subjectId = id;
  form.tagIds = [];
  tags.value = id ? await catalogApi.tags(id).catch(() => []) : [];
}

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    reset();
    await loadMaster();
    const c = props.cls;
    Object.assign(form, {
      subjectId: c?.subjectId ?? master.value?.subjects[0]?.id ?? null,
      gradeIds: [...(c?.gradeIds ?? [])],
      tagIds: [...(c?.tagIds ?? [])],
      title: c?.title ?? '',
      description: c?.description ?? '',
      mode: c?.mode ?? 'self_paced',
      visibility: c?.visibility ?? 'public',
      enrollment: c?.enrollment ?? 'open',
    });
    tags.value = form.subjectId ? await catalogApi.tags(form.subjectId).catch(() => []) : [];
  },
);

async function save() {
  const res = await run(() => (props.cls ? teachingApi.updateClass(props.cls.id, { ...form }) : teachingApi.createClass({ ...form })));
  if (res.ok) {
    emit('saved', res.data);
    close();
  }
}

function onOk(e) {
  e.preventDefault();
  save();
}

function close() {
  emit('update:open', false);
}
</script>

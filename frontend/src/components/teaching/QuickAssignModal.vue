<template>
  <hui-modal :open="open" :title="`Giao nhanh: ${refTitle}`" ok-text="Giao bài" cancel-text="Hủy" :confirmLoading="loading" @hui-ok="onOk" @hui-close="close">
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <p class="muted small" style="margin: 0">
        {{ kind === 'exam' ? 'Đề' : 'Bài tập' }} được thêm vào cuối lộ trình đã xuất bản của lớp (phần lộ trình đang sửa dở không bị xuất bản theo),
        học sinh được giao nhận thông báo ngay.
      </p>
      <label class="field">
        <span>Lớp học</span>
        <select v-model="form.classId" class="native-select" @change="loadAudience">
          <option value="" disabled>— Chọn lớp —</option>
          <option v-for="c in classes" :key="c.id" :value="c.id">{{ c.title }}</option>
        </select>
        <span v-if="fields.classId" class="field-error">{{ fields.classId }}</span>
      </label>
      <div class="row">
        <label v-for="(label, key) in AUDIENCE" :key="key" class="small inline">
          <input v-model="form.audience" type="radio" :value="key" /> {{ label }}
        </label>
      </div>
      <div v-if="form.audience === 'groups'" class="pick">
        <label v-for="g in groups" :key="g.id" class="small"><input v-model="form.groupIds" type="checkbox" :value="g.id" /> {{ g.name }} ({{ g.memberCount }})</label>
        <span v-if="!groups.length" class="muted small">Chưa có nhóm học sinh.</span>
        <span v-if="fields.groupIds" class="field-error">{{ fields.groupIds }}</span>
      </div>
      <div v-if="form.audience === 'students'" class="pick">
        <label v-for="m in students" :key="m.userId" class="small"><input v-model="form.userIds" type="checkbox" :value="m.userId" /> {{ m.fullName }}</label>
        <span v-if="!students.length" class="muted small">{{ form.classId ? 'Lớp chưa có học sinh.' : 'Chọn lớp trước.' }}</span>
        <span v-if="fields.userIds" class="field-error">{{ fields.userIds }}</span>
      </div>
      <label class="field">
        <span>{{ kind === 'exam' ? 'Hạn làm (quá hạn không bắt đầu làm được)' : 'Hạn nộp (nộp muộn vẫn nhận, đánh dấu muộn)' }}</span>
        <input v-model="form.due" type="datetime-local" class="native-input" />
        <span v-if="fields.dueAt" class="field-error">{{ fields.dueAt }}</span>
      </label>
      <hui-input v-model="form.title" label="Tên hiện trong lộ trình (bỏ trống = tên gốc)" :error="fields.title" />
      <p v-for="k in ['examId', 'assignmentId', 'kind']" :key="k" class="field-error">{{ fields[k] }}</p>
    </form>
  </hui-modal>
</template>

<script setup>
/** Giao nhanh một đề / bài tập cho lớp, nhóm hoặc học sinh (M41). */
import { message } from '@itmrhoang/h-ui/hui-message';
import { reactive, ref, watch } from 'vue';

import { teachingApi } from '../../api';
import { useSubmit } from '../../utils/form';

const AUDIENCE = { all: 'Cả lớp', groups: 'Nhóm học sinh', students: 'Một vài học sinh' };

const props = defineProps({
  open: { type: Boolean, default: false },
  /** 'exam' | 'homework' */
  kind: { type: String, default: 'exam' },
  refId: { type: String, default: '' },
  refTitle: { type: String, default: '' },
});
const emit = defineEmits(['update:open', 'assigned']);

const { loading, error, fields, run, reset } = useSubmit();
const classes = ref([]);
const groups = ref([]);
const students = ref([]);
const form = reactive({ classId: '', audience: 'all', groupIds: [], userIds: [], due: '', title: '' });

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    reset();
    Object.assign(form, { audience: 'all', groupIds: [], userIds: [], due: '', title: '' });
    const [cs, gs] = await Promise.all([
      teachingApi.classes({ pageSize: 100 }).catch(() => ({ items: [] })),
      teachingApi.groups().catch(() => []),
    ]);
    classes.value = cs.items.filter((c) => !c.archivedAt && c.myRole !== 'assistant');
    groups.value = gs;
    if (!classes.value.some((c) => c.id === form.classId)) form.classId = classes.value[0]?.id ?? '';
    loadAudience();
  },
);

async function loadAudience() {
  form.userIds = [];
  students.value = form.classId ? await teachingApi.members(form.classId, 'active').catch(() => []) : [];
}

function onOk(e) {
  e.preventDefault();
  save();
}

async function save() {
  if (!form.classId) {
    fields.classId = 'Chọn lớp học.';
    return;
  }
  const body = {
    kind: props.kind,
    examId: props.kind === 'exam' ? props.refId : null,
    assignmentId: props.kind === 'homework' ? props.refId : null,
    title: form.title.trim(),
    dueAt: form.due ? new Date(form.due).toISOString() : null,
    groupIds: form.audience === 'groups' ? form.groupIds : [],
    userIds: form.audience === 'students' ? form.userIds : [],
  };
  const res = await run(() => teachingApi.quickAssign(form.classId, body));
  if (!res.ok) return;
  const cls = classes.value.find((c) => c.id === form.classId)?.title ?? '';
  message.success(`Đã giao cho ${res.data.assigned} học sinh lớp "${cls}".`);
  emit('assigned', res.data);
  close();
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.pick {
  display: flex;
  flex-wrap: wrap;
  gap: 6px 14px;
  max-height: 180px;
  overflow: auto;
  padding: 8px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.inline {
  display: inline-flex;
  gap: 4px;
  align-items: center;
}
</style>

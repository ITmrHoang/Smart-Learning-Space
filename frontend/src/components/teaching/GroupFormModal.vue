<template>
  <hui-modal :open="open" :title="groupId ? 'Sửa nhóm học sinh' : 'Tạo nhóm học sinh'" size="lg" ok-text="Lưu" cancel-text="Hủy" :confirmLoading="loading" @hui-ok="onOk" @hui-close="close">
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input v-model="form.name" label="Tên nhóm" placeholder="Ví dụ: Nhóm ôn thi chuyên Toán" :error="fields.name" required />
      <hui-select label="Gắn với lớp học (không bắt buộc)" :options="classOptions" :value="form.classId" placeholder="Không gắn lớp học" clearable @change="onClass($event.detail.value)" />
      <hui-select
        label="Thành viên"
        mode="multiple"
        searchable
        :placeholder="form.classId ? 'Chọn học sinh của lớp học này' : 'Chọn học sinh từ các lớp học của bạn'"
        :options="studentOptions"
        :value="form.memberIds"
        :error="fields.memberIds"
        @change="form.memberIds = $event.detail.value"
      />
      <p class="muted small">
        Chỉ chọn được học sinh đang học trong lớp học của bạn. Mời nhóm vào buổi học: thêm / bớt thành viên sau này thì buổi học cũng cập nhật theo.
        Đã chọn {{ form.memberIds?.length ?? 0 }} học sinh.
      </p>
    </form>
  </hui-modal>
</template>

<script setup>
import { computed, reactive, ref, watch } from 'vue';

import { teachingApi } from '../../api';
import { useSubmit } from '../../utils/form';

const props = defineProps({
  open: { type: Boolean, default: false },
  /** ID nhóm khi sửa */
  groupId: { type: String, default: null },
});
const emit = defineEmits(['update:open', 'saved']);
const { loading, error, fields, run, reset } = useSubmit();
const classes = ref([]);
const students = ref([]);
/** id → nhãn của học sinh đã gặp (giữ tên thành viên không thuộc lớp học đang lọc) */
const known = reactive(new Map());
const form = reactive({ name: '', classId: null, memberIds: [] });

const label = (s) => `${s.fullName} (@${s.username})`;
const classOptions = computed(() => classes.value.map((c) => ({ value: c.id, label: c.title })));
const studentOptions = computed(() => {
  const list = students.value.map((s) => ({ value: s.id, label: label(s) }));
  for (const id of form.memberIds) if (!list.some((o) => o.value === id)) list.push({ value: id, label: known.get(id) ?? id });
  return list;
});

async function loadStudents() {
  students.value = await teachingApi.students(form.classId ? { classId: form.classId } : {}).catch(() => []);
  for (const s of students.value) known.set(s.id, label(s));
}

async function onClass(id) {
  form.classId = id || null;
  await loadStudents();
}

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    reset();
    Object.assign(form, { name: '', classId: null, memberIds: [] });
    const [cs, g] = await Promise.all([
      teachingApi.classes({ pageSize: 100 }).catch(() => ({ items: [] })),
      props.groupId ? teachingApi.group(props.groupId) : null,
    ]);
    classes.value = cs.items;
    if (g) {
      for (const m of g.members) known.set(m.id, label(m));
      Object.assign(form, { name: g.name, classId: g.classId, memberIds: g.members.map((m) => m.id) });
    }
    await loadStudents();
  },
);

async function save() {
  const body = { name: form.name.trim(), classId: form.classId, memberIds: form.memberIds };
  const res = await run(() => (props.groupId ? teachingApi.updateGroup(props.groupId, body) : teachingApi.createGroup(body)));
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

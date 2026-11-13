<template>
  <hui-modal :open="open" :title="thread ? 'Sửa câu hỏi' : 'Đặt câu hỏi'" size="lg" ok-text="Gửi" cancel-text="Hủy" :confirmLoading="loading" @hui-ok="onOk" @hui-close="close">
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-select label="Về bước nào trong lộ trình" :options="stepOptions" :value="form.stepId" placeholder="Chung cho cả lớp học" clearable :error="fields.stepId" @change="form.stepId = $event.detail.value || null" />
      <hui-input v-model="form.title" label="Tiêu đề" placeholder="Ví dụ: Vì sao bước 2 lại đổi dấu?" :error="fields.title" required />
      <MarkdownEditor v-model="form.body" label="Nội dung câu hỏi" :rows="6" :error="fields.body" compact placeholder="Viết rõ chỗ chưa hiểu; công thức viết trong $…$" />
      <p class="muted small" style="margin: 0">Câu hỏi hiện cho cả lớp học và giáo viên. Không đăng thông tin cá nhân (số điện thoại, địa chỉ…).</p>
    </form>
  </hui-modal>
</template>

<script setup>
import { computed, reactive, watch } from 'vue';

import { qaApi } from '../../api';
import { useSubmit } from '../../utils/form';
import MarkdownEditor from '../teaching/MarkdownEditor.vue';

const props = defineProps({
  open: { type: Boolean, default: false },
  classId: { type: String, required: true },
  /** Các bước (bài / đề / buổi / thẻ) để gắn câu hỏi: [{ id, title }] */
  steps: { type: Array, default: () => [] },
  /** Bước chọn sẵn (mở từ trang bài học) */
  stepId: { type: String, default: null },
  /** ThreadDetail khi sửa */
  thread: { type: Object, default: null },
});
const emit = defineEmits(['update:open', 'saved']);
const { loading, error, fields, run, reset } = useSubmit();
const form = reactive({ stepId: null, title: '', body: '' });
const stepOptions = computed(() => props.steps.map((s, i) => ({ value: s.id, label: `${i + 1}. ${s.title}` })));

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    reset();
    const t = props.thread;
    Object.assign(form, { stepId: t?.stepId ?? props.stepId ?? null, title: t?.title ?? '', body: t?.body ?? '' });
  },
);

async function save() {
  const body = { stepId: form.stepId || null, title: form.title.trim(), body: form.body.trim() };
  const res = await run(() => (props.thread ? qaApi.update(props.thread.id, body) : qaApi.create(props.classId, body)));
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

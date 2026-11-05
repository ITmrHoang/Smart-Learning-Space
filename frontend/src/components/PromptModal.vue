<template>
  <hui-modal :open="open" :title="title" size="sm" ok-text="Lưu" cancel-text="Hủy" :confirmLoading="loading" @hui-ok="onOk" @hui-close="close">
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input v-model="value" :label="label" :error="Object.values(fields)[0]" required />
    </form>
  </hui-modal>
</template>

<script setup>
/** Hộp nhập một dòng chữ (tên thư mục, tên lớp học khi sao chép…). */
import { ref, watch } from 'vue';

import { useSubmit } from '../utils/form';

const props = defineProps({
  open: { type: Boolean, default: false },
  title: { type: String, default: '' },
  label: { type: String, default: 'Tên' },
  initial: { type: String, default: '' },
  /** async (value) => any */
  saveFn: { type: Function, required: true },
});
const emit = defineEmits(['update:open', 'saved']);
const value = ref('');
const { loading, error, fields, run, reset } = useSubmit();

watch(
  () => props.open,
  (open) => {
    if (open) {
      reset();
      value.value = props.initial;
    }
  },
);

async function save() {
  const res = await run(() => props.saveFn(value.value.trim()));
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

<template>
  <hui-modal
    :open="open"
    :title="title"
    ok-text="Lưu"
    cancel-text="Hủy"
    :confirmLoading="loading"
    @hui-ok="onOk"
    @hui-close="close"
  >
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input v-model="form.name" label="Tên" :error="fields.name" required />
      <hui-input-number
        :value="form.sortOrder"
        label="Thứ tự hiển thị"
        :min="0"
        :error="fields.sortOrder"
        @change="form.sortOrder = Number($event.detail.value)"
      />
      <hui-switch :checked="form.isActive" label="Đang dùng" @change="form.isActive = $event.detail.checked" />
    </form>
  </hui-modal>
</template>

<script setup>
/** Sửa một mục master data (môn học / lớp / danh mục): tên, thứ tự, đang dùng. */
import { reactive, watch } from 'vue';

import { useSubmit } from '../utils/form';

const props = defineProps({
  open: { type: Boolean, default: false },
  title: { type: String, default: 'Sửa' },
  item: { type: Object, default: null },
  /** async (changes) => item đã lưu */
  saveFn: { type: Function, required: true },
});
const emit = defineEmits(['update:open', 'saved']);

const form = reactive({ name: '', sortOrder: 0, isActive: true });
const { loading, error, fields, run, reset } = useSubmit();

watch(
  () => [props.open, props.item],
  () => {
    if (props.open && props.item) {
      reset();
      Object.assign(form, { name: props.item.name, sortOrder: props.item.sortOrder, isActive: props.item.isActive });
    }
  },
  { immediate: true },
);

async function save() {
  const changes = {};
  if (form.name.trim() !== props.item.name) changes.name = form.name.trim();
  if (form.sortOrder !== props.item.sortOrder) changes.sortOrder = form.sortOrder;
  if (form.isActive !== props.item.isActive) changes.isActive = form.isActive;
  const res = await run(() => props.saveFn(changes));
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

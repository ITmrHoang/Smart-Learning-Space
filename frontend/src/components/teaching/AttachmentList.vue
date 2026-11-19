<template>
  <div class="attachments">
    <div class="row head">
      <b class="small">{{ label }}</b>
      <template v-if="editable">
        <hui-button size="sm" variant="outline" :loading="uploading" :disabled="modelValue.length >= MAX" @hui-click="input?.click()">
          {{ uploading ? `Đang tải ${progress}%` : 'Thêm tài liệu' }}
        </hui-button>
        <input ref="input" type="file" :accept="ATTACHMENT_ACCEPT" multiple hidden @change="onPick" />
        <span class="muted small">PDF, Word, Excel, PowerPoint · tối đa 20 MB / file, {{ MAX }} file</span>
      </template>
    </div>
    <p v-if="!modelValue.length" class="muted small">{{ editable ? 'Chưa có tài liệu.' : 'Không có tài liệu.' }}</p>
    <ul class="clean-list">
      <li v-for="f in modelValue" :key="f.id" class="row file">
        <span class="kind">{{ kindOf(f) }}</span>
        <a :href="f.url" target="_blank" rel="noopener">{{ f.name }}</a>
        <span class="muted small">{{ formatBytes(f.size) }}</span>
        <hui-button v-if="editable" size="sm" variant="text" @hui-click="remove(f)">Bỏ</hui-button>
      </li>
    </ul>
    <p v-if="error" class="field-error">{{ error }}</p>
  </div>
</template>

<script setup>
/** Tài liệu đính kèm bài học / giáo án (M37): upload thẳng lên Garage, link có chữ ký; PDF mở trong tab mới. */
import { ref } from 'vue';

import { ATTACHMENT_ACCEPT, DOCX, PPTX, XLSX, formatBytes, uploadFile } from '../../utils/upload';

const MAX = 10;
const KINDS = { 'application/pdf': 'PDF', [DOCX]: 'Word', [XLSX]: 'Excel', [PPTX]: 'PowerPoint' };

const props = defineProps({
  /** [{ id, name, contentType, size, url }] */
  modelValue: { type: Array, default: () => [] },
  editable: { type: Boolean, default: false },
  label: { type: String, default: 'Tài liệu đính kèm' },
});
const emit = defineEmits(['update:modelValue']);
const input = ref(null);
const uploading = ref(false);
const progress = ref(0);
const error = ref('');

const kindOf = (f) => KINDS[f.contentType] ?? 'File';

async function onPick(e) {
  const files = [...(e.target.files ?? [])];
  e.target.value = '';
  error.value = '';
  uploading.value = true;
  try {
    let list = [...props.modelValue];
    for (const file of files.slice(0, MAX - list.length)) {
      progress.value = 0;
      const saved = await uploadFile(file, 'attachment', { onProgress: (p) => (progress.value = p) });
      list = [...list, saved];
      emit('update:modelValue', list);
    }
  } catch (err) {
    error.value = err.message;
  } finally {
    uploading.value = false;
  }
}

function remove(f) {
  emit(
    'update:modelValue',
    props.modelValue.filter((x) => x.id !== f.id),
  );
}
</script>

<style scoped>
.attachments {
  display: grid;
  gap: 6px;
}
.head {
  flex-wrap: wrap;
}
.file {
  gap: 10px;
  padding: 4px 0;
}
.kind {
  min-width: 78px;
  padding: 1px 6px;
  border-radius: 4px;
  background: var(--hui-bg-subtle, #f3f4f6);
  font-size: 0.75rem;
  text-align: center;
}
</style>

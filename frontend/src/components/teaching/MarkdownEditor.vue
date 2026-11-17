<template>
  <div class="md-editor" :class="{ compact }">
    <div class="md-editor__head">
      <span class="md-editor__label">{{ label }}</span>
      <span class="md-editor__tools">
        <span class="muted small">Markdown · công thức $…$</span>
        <hui-button size="sm" variant="text" title="Chèn khối đồ thị hàm số (mỗi dòng một đồ thị)" @hui-click="insertPlot">
          Chèn đồ thị
        </hui-button>
        <template v-if="images">
          <hui-button size="sm" variant="text" :loading="uploading" @hui-click="fileInput?.click()">
            {{ uploading ? `Đang tải ảnh ${progress}%` : 'Chèn ảnh' }}
          </hui-button>
          <input ref="fileInput" type="file" accept="image/png,image/jpeg,image/gif,image/webp" hidden @change="onPick" />
        </template>
      </span>
    </div>
    <div class="md-editor__body">
      <textarea
        ref="textarea"
        class="md-editor__input"
        :class="{ dragging }"
        :rows="rows"
        :value="modelValue"
        :placeholder="placeholder"
        @input="$emit('update:modelValue', $event.target.value)"
        @paste="onPaste"
        @dragover.prevent="dragging = images"
        @dragleave="dragging = false"
        @drop="onDrop"
      />
      <div class="md-editor__preview" aria-label="Xem trước">
        <hui-markdown v-if="modelValue.trim()" :value="modelValue" />
        <span v-else class="muted small">Xem trước hiện ở đây</span>
      </div>
    </div>
    <p v-if="images" class="muted small hint">Dán (Ctrl + V) hoặc kéo ảnh vào ô soạn để chèn ảnh (tối đa 5 MB).</p>
    <p v-if="error" class="field-error">{{ error }}</p>
  </div>
</template>

<script setup>
/** Ô soạn Markdown + công thức, xem trước ngay bên cạnh (dùng hui-markdown); chèn ảnh lên Garage. */
import { message } from '@itmrhoang/h-ui/hui-message';
import { nextTick, ref } from 'vue';

import { imageMarkdown, uploadFile } from '../../utils/upload';

const props = defineProps({
  modelValue: { type: String, default: '' },
  label: { type: String, default: 'Nội dung' },
  placeholder: { type: String, default: '' },
  rows: { type: Number, default: 10 },
  error: { type: String, default: '' },
  /** Xem trước nằm dưới thay vì bên cạnh (dùng trong modal) */
  compact: { type: Boolean, default: false },
  /** Cho chèn ảnh (upload lên Garage) */
  images: { type: Boolean, default: true },
});
const emit = defineEmits(['update:modelValue']);

const textarea = ref(null);
const fileInput = ref(null);
const uploading = ref(false);
const progress = ref(0);
const dragging = ref(false);

/** Chèn chữ vào vị trí con trỏ (mỗi ảnh một dòng riêng). */
async function insertAtCursor(text) {
  const el = textarea.value;
  const value = props.modelValue;
  const start = el?.selectionStart ?? value.length;
  const end = el?.selectionEnd ?? value.length;
  const before = value.slice(0, start);
  const after = value.slice(end);
  const lead = before && !before.endsWith('\n') ? '\n' : '';
  const trail = after.startsWith('\n') ? '' : '\n';
  emit('update:modelValue', `${before}${lead}${text}${trail}${after}`);
  await nextTick();
  const pos = before.length + lead.length + text.length + trail.length;
  el?.focus();
  el?.setSelectionRange(pos, pos);
}

/** Khối ```plot mẫu: hiện thành hui-plot (đồ thị kéo / phóng to được, tham số có thanh trượt). */
const PLOT_TEMPLATE = '```plot\ny = 2x + 1\ny = x^2 - 3\nA(1; 3)\n```';

function insertPlot() {
  insertAtCursor(PLOT_TEMPLATE);
}

async function upload(files) {
  const images = [...files].filter((f) => f.type.startsWith('image/'));
  if (!images.length || uploading.value) return;
  uploading.value = true;
  try {
    for (const f of images) {
      progress.value = 0;
      const saved = await uploadFile(f, 'content', { onProgress: (p) => (progress.value = p) });
      await insertAtCursor(imageMarkdown(saved));
    }
  } catch (e) {
    message.error(e.message);
  } finally {
    uploading.value = false;
  }
}

function onPick(e) {
  upload(e.target.files);
  e.target.value = '';
}

function onPaste(e) {
  if (!props.images) return;
  const files = [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith('image/'));
  if (!files.length) return;
  e.preventDefault();
  upload(files);
}

function onDrop(e) {
  dragging.value = false;
  if (!props.images || !e.dataTransfer?.files?.length) return;
  e.preventDefault();
  upload(e.dataTransfer.files);
}
</script>

<style scoped>
.md-editor {
  display: grid;
  gap: 6px;
}
.md-editor__head {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: space-between;
  align-items: center;
}
.md-editor__tools {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 4px 8px;
  align-items: center;
}
.md-editor__label {
  font-weight: 500;
  font-size: 0.9rem;
}
.md-editor__body {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.compact .md-editor__body {
  grid-template-columns: 1fr;
}
.md-editor__input {
  font: inherit;
  font-family: var(--hui-font-mono, ui-monospace, Consolas, monospace);
  font-size: 0.9rem;
  line-height: 1.5;
  padding: 10px 12px;
  border: 1px solid var(--hui-border, #d9d9d9);
  border-radius: var(--hui-radius-md, 8px);
  resize: vertical;
  min-height: 80px;
  background: var(--hui-bg-container, #fff);
  color: inherit;
}
.md-editor__input:focus {
  outline: 2px solid var(--hui-primary, #1677ff);
  outline-offset: -1px;
}
.md-editor__input.dragging {
  outline: 2px dashed var(--hui-primary, #1677ff);
  outline-offset: -2px;
}
.md-editor__preview {
  border: 1px dashed var(--hui-border, #d9d9d9);
  border-radius: var(--hui-radius-md, 8px);
  padding: 10px 12px;
  overflow: auto;
  max-height: 480px;
  background: var(--hui-bg-container, #fff);
}
.hint {
  margin: 0;
}
@media (max-width: 860px) {
  .md-editor__body {
    grid-template-columns: 1fr;
  }
}
</style>

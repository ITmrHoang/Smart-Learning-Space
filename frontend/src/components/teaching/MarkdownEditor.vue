<template>
  <div class="md-editor" :class="{ compact }">
    <div class="md-editor__head">
      <span class="md-editor__label">{{ label }}</span>
      <span class="muted small">Markdown · công thức $…$ · ảnh ![mô tả](link)</span>
    </div>
    <div class="md-editor__body">
      <textarea
        class="md-editor__input"
        :rows="rows"
        :value="modelValue"
        :placeholder="placeholder"
        @input="$emit('update:modelValue', $event.target.value)"
      />
      <div class="md-editor__preview" aria-label="Xem trước">
        <hui-markdown v-if="modelValue.trim()" :value="modelValue" />
        <span v-else class="muted small">Xem trước hiện ở đây</span>
      </div>
    </div>
    <p v-if="error" class="field-error">{{ error }}</p>
  </div>
</template>

<script setup>
/** Ô soạn Markdown + công thức, xem trước ngay bên cạnh (dùng hui-markdown). */
defineProps({
  modelValue: { type: String, default: '' },
  label: { type: String, default: 'Nội dung' },
  placeholder: { type: String, default: '' },
  rows: { type: Number, default: 10 },
  error: { type: String, default: '' },
  /** Xem trước nằm dưới thay vì bên cạnh (dùng trong modal) */
  compact: { type: Boolean, default: false },
});
defineEmits(['update:modelValue']);
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
  align-items: baseline;
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
.md-editor__preview {
  border: 1px dashed var(--hui-border, #d9d9d9);
  border-radius: var(--hui-radius-md, 8px);
  padding: 10px 12px;
  overflow: auto;
  max-height: 480px;
  background: var(--hui-bg-container, #fff);
}
@media (max-width: 860px) {
  .md-editor__body {
    grid-template-columns: 1fr;
  }
}
</style>

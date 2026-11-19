<template>
  <div class="snippets">
    <div class="row between">
      <span class="small muted">Nhận xét mẫu — bấm để chèn</span>
      <hui-button size="sm" variant="text" :disabled="!canSave" :loading="saving" @hui-click="save">＋ Lưu nhận xét đang viết</hui-button>
    </div>
    <div v-if="list.length" class="chips">
      <span v-for="s in list" :key="s.id" class="chip">
        <button type="button" class="chip-text" :title="s.body" @click="$emit('insert', s.body)">{{ s.body }}</button>
        <button type="button" class="chip-del" :aria-label="`Xóa nhận xét mẫu: ${s.body}`" title="Xóa" @click="remove(s)">×</button>
      </span>
    </div>
    <p v-else-if="loaded" class="small muted" style="margin: 0">
      Chưa có. Viết nhận xét hay dùng ("Trình bày rõ ràng", "Thiếu điều kiện xác định"…) rồi bấm "Lưu nhận xét đang viết".
    </p>
  </div>
</template>

<script setup>
/** Nhận xét mẫu của giáo viên (dùng chung khi chấm câu tự luận và chấm bài tập). Phát `insert` với nội dung. */
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref } from 'vue';

import { teachingApi } from '../../api';

const props = defineProps({
  /** Nhận xét đang viết (để lưu làm mẫu) */
  text: { type: String, default: '' },
});
defineEmits(['insert']);

const list = ref([]);
const loaded = ref(false);
const saving = ref(false);
const canSave = computed(() => {
  const t = props.text.trim();
  return t.length > 0 && t.length <= 1000 && !list.value.some((s) => s.body === t);
});

onMounted(async () => {
  try {
    list.value = await teachingApi.snippets();
  } catch {
    list.value = [];
  } finally {
    loaded.value = true;
  }
});

async function save() {
  saving.value = true;
  try {
    list.value.push(await teachingApi.createSnippet(props.text.trim()));
    message.success('Đã lưu nhận xét mẫu.');
  } catch (e) {
    message.error(e.message);
  } finally {
    saving.value = false;
  }
}

async function remove(s) {
  if (!window.confirm(`Xóa nhận xét mẫu "${s.body}"?`)) return;
  try {
    await teachingApi.deleteSnippet(s.id);
    list.value = list.value.filter((x) => x.id !== s.id);
  } catch (e) {
    message.error(e.message);
  }
}
</script>

<style scoped>
.snippets {
  display: grid;
  gap: 6px;
}
.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.chip {
  display: inline-flex;
  align-items: center;
  max-width: 100%;
  border: 1px solid var(--hui-border, #d9d9d9);
  border-radius: 999px;
  background: var(--hui-bg-container, #fff);
  font-size: 0.8125rem;
}
.chip-text {
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  border: 0;
  background: none;
  padding: 3px 4px 3px 10px;
  font: inherit;
  cursor: pointer;
}
.chip-text:hover {
  color: var(--hui-primary, #1677ff);
}
.chip-del {
  border: 0;
  background: none;
  padding: 0 8px 0 2px;
  color: var(--hui-text-secondary, #888);
  cursor: pointer;
}
.chip-del:hover {
  color: var(--hui-danger, #d4380d);
}
</style>

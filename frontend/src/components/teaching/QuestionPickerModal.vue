<template>
  <hui-modal :open="open" title="Chọn câu hỏi từ ngân hàng" size="lg" ok-text="Thêm vào phần" cancel-text="Hủy" @hui-ok="confirm" @hui-close="close">
    <div class="stack">
      <div class="toolbar">
        <hui-input class="grow" type="search" label="Tìm" clearable @hui-search="list.setFilter('q', $event.detail.value)" @clear="list.setFilter('q', '')" />
        <hui-select label="Dạng" allow-clear placeholder="Tất cả" :options="kindOptions" @change="list.setFilter('kind', $event.detail.value)" />
        <hui-select label="Mức độ" allow-clear placeholder="Tất cả" :options="levelOptions" @change="list.setFilter('level', $event.detail.value)" />
      </div>
      <p class="muted small">Đã chọn {{ picked.size }} câu · {{ list.total.value }} câu trong môn này</p>
      <ul class="clean-list picker">
        <li v-for="q in list.items.value" :key="q.id" :class="{ disabled: exclude.includes(q.id) }">
          <label>
            <input type="checkbox" :disabled="exclude.includes(q.id)" :checked="picked.has(q.id) || exclude.includes(q.id)" @change="toggle(q, $event.target.checked)" />
            <span class="stack" style="gap: 4px">
              <span class="row">
                <hui-tag size="sm">{{ QUESTION_KIND_LABELS[q.kind] }}</hui-tag>
                <hui-tag size="sm" :color="LEVEL_COLORS[q.level]">{{ LEVEL_LABELS[q.level] }}</hui-tag>
                <span v-if="exclude.includes(q.id)" class="muted small">đã có trong đề</span>
              </span>
              <hui-markdown :value="q.content" />
            </span>
          </label>
        </li>
      </ul>
      <hui-pagination :total="list.total.value" :current="list.page.value" :pageSize="list.pageSize.value" hide-on-single-page @change="list.onPage($event.detail)" />
    </div>
  </hui-modal>
</template>

<script setup>
import { reactive, watch } from 'vue';

import { teachingApi } from '../../api';
import { usePagedList } from '../../utils/list';
import { LEVEL_COLORS, LEVEL_LABELS, QUESTION_KIND_LABELS, toOptions } from '../../utils/teaching';

const props = defineProps({
  open: { type: Boolean, default: false },
  subjectId: { type: String, default: null },
  /** ID câu đã có trong đề */
  exclude: { type: Array, default: () => [] },
});
const emit = defineEmits(['update:open', 'picked']);
const list = usePagedList((q) => teachingApi.questions(q), { subjectId: '', q: '', kind: '', level: '' }, { pageSize: 10 });
const picked = reactive(new Map());
const kindOptions = toOptions(QUESTION_KIND_LABELS);
const levelOptions = toOptions(LEVEL_LABELS);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    picked.clear();
    list.setFilter('subjectId', props.subjectId || '');
  },
);

function toggle(q, checked) {
  if (checked) picked.set(q.id, q);
  else picked.delete(q.id);
}

function confirm(e) {
  e.preventDefault();
  emit('picked', [...picked.values()]);
  close();
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.picker {
  max-height: 440px;
  overflow: auto;
}
.picker li {
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.picker li.disabled {
  opacity: 0.6;
}
.picker label {
  display: flex;
  gap: 10px;
  padding: 10px;
  cursor: pointer;
}
</style>

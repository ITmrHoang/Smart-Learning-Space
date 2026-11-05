<template>
  <hui-modal :open="open" title="Nhập câu hỏi từ Excel" size="lg" no-footer @hui-close="close">
    <div class="stack">
      <hui-alert v-if="error" severity="error" :description="error" />
      <p class="muted small">
        Tải file mẫu, điền câu hỏi (công thức gõ dạng $…$), rồi chọn file để xem trước. Hệ thống chỉ lưu khi bạn bấm "Lưu".
        Tối đa 500 câu / file, dung lượng dưới 1 MB.
      </p>
      <div class="grid-3">
        <hui-select label="Môn học" :options="subjectOptions" :value="subjectId" @change="subjectId = $event.detail.value" />
        <hui-select label="Lớp (không bắt buộc)" allow-clear :options="gradeOptions" :value="gradeId" @change="gradeId = $event.detail.value || null" />
        <hui-select label="Đưa vào thư mục" allow-clear :options="folderOptions" :value="folderId" @change="folderId = $event.detail.value || null" />
      </div>
      <div class="row">
        <hui-button variant="outline" :loading="downloading" @hui-click="downloadTemplate">Tải file mẫu</hui-button>
        <label class="file-btn">
          <input type="file" accept=".xlsx" :disabled="!subjectId || previewing" @change="onFile" />
          <span>{{ previewing ? 'Đang đọc…' : 'Chọn file .xlsx' }}</span>
        </label>
        <span v-if="fileName" class="muted small">{{ fileName }}</span>
      </div>

      <template v-if="preview">
        <hui-alert
          :severity="invalid.length ? 'warning' : 'success'"
          :description="`Đọc được ${preview.items.length} câu: ${preview.validCount} câu hợp lệ${invalid.length ? `, ${invalid.length} câu lỗi (sẽ bỏ qua)` : ''}.`"
        />
        <div class="preview">
          <table>
            <thead>
              <tr>
                <th>Dòng</th>
                <th>Dạng</th>
                <th>Nội dung</th>
                <th>Ghi chú</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="it in preview.items" :key="it.row" :class="{ bad: it.errors.length }">
                <td>{{ it.row }}</td>
                <td>{{ QUESTION_KIND_LABELS[it.kind] ?? '?' }}<br /><span class="muted small">{{ LEVEL_LABELS[it.level] }}</span></td>
                <td>
                  <hui-markdown :value="it.content" />
                  <ol v-if="it.options.length" type="A" class="small">
                    <li v-for="(o, i) in it.options" :key="i" :class="{ ok: o.correct }">
                      <hui-markdown inline :value="o.content" /> <b v-if="o.correct">✓</b>
                    </li>
                  </ol>
                  <p v-if="it.accepted.length" class="small">Đáp án: {{ it.accepted.join(' · ') }}</p>
                </td>
                <td class="small">
                  <div v-for="e in it.errors" :key="e" class="err">{{ e }}</div>
                  <div v-for="w in it.warnings" :key="w" class="muted">{{ w }}</div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="form-actions">
          <hui-button variant="outline" @hui-click="preview = null">Chọn file khác</hui-button>
          <hui-button :loading="saving" :disabled="!preview.validCount" @hui-click="save">Lưu {{ preview.validCount }} câu</hui-button>
        </div>
      </template>
    </div>
  </hui-modal>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, ref, watch } from 'vue';

import { teachingApi } from '../../api';
import { useMasterData } from '../../utils/masterData';
import { LEVEL_LABELS, QUESTION_KIND_LABELS } from '../../utils/teaching';

const props = defineProps({
  open: { type: Boolean, default: false },
  defaultSubjectId: { type: String, default: null },
  defaultFolderId: { type: String, default: null },
  folders: { type: Array, default: () => [] },
});
const emit = defineEmits(['update:open', 'saved']);

const { data: master, load: loadMaster } = useMasterData();
const subjectId = ref(null);
const gradeId = ref(null);
const folderId = ref(null);
const preview = ref(null);
const fileName = ref('');
const error = ref('');
const previewing = ref(false);
const saving = ref(false);
const downloading = ref(false);

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const gradeOptions = computed(() => (master.value?.grades ?? []).map((g) => ({ value: g.id, label: g.name })));
const folderOptions = computed(() => props.folders.map((f) => ({ value: f.id, label: f.path ?? f.name })));
const invalid = computed(() => preview.value?.items.filter((i) => i.errors.length) ?? []);

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    await loadMaster();
    subjectId.value = props.defaultSubjectId || master.value?.subjects[0]?.id || null;
    folderId.value = props.defaultFolderId;
    gradeId.value = null;
    preview.value = null;
    fileName.value = '';
    error.value = '';
  },
);

async function downloadTemplate() {
  downloading.value = true;
  try {
    const blob = await teachingApi.importTemplate();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'mau-cau-hoi.xlsx';
    a.click();
    URL.revokeObjectURL(url);
  } catch (e) {
    error.value = e.message;
  } finally {
    downloading.value = false;
  }
}

async function onFile(e) {
  const file = e.target.files?.[0];
  e.target.value = '';
  if (!file) return;
  error.value = '';
  if (file.size > 1024 * 1024) {
    error.value = 'File lớn hơn 1 MB. Hãy chia thành nhiều file.';
    return;
  }
  fileName.value = file.name;
  previewing.value = true;
  try {
    preview.value = await teachingApi.importPreview(subjectId.value, file);
  } catch (err) {
    error.value = err.message;
    preview.value = null;
  } finally {
    previewing.value = false;
  }
}

async function save() {
  saving.value = true;
  error.value = '';
  try {
    const items = preview.value.items
      .filter((i) => !i.errors.length)
      .map((i) => ({
        subjectId: subjectId.value,
        gradeId: gradeId.value,
        folderId: folderId.value,
        kind: i.kind,
        level: i.level,
        tagIds: i.tagIds,
        content: i.content,
        options: i.options,
        accepted: i.accepted,
        explanation: i.explanation,
      }));
    const res = await teachingApi.bulkCreateQuestions(items);
    message.success(`Đã lưu ${res.created} câu hỏi.`);
    emit('saved');
    close();
  } catch (e) {
    error.value = e.fields?.items?.[0] ?? e.message;
  } finally {
    saving.value = false;
  }
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.file-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  padding: 6px 14px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-primary, #1677ff);
  color: #fff;
  cursor: pointer;
  font-size: 0.9rem;
}
.file-btn input {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
}
.preview {
  max-height: 420px;
  overflow: auto;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
table {
  border-collapse: collapse;
  width: 100%;
}
th,
td {
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
  padding: 8px;
  text-align: left;
  vertical-align: top;
}
tr.bad {
  background: color-mix(in srgb, var(--hui-danger, #ff4d4f) 7%, transparent);
}
.err {
  color: var(--hui-danger, #d4380d);
}
.ok {
  font-weight: 600;
}
</style>

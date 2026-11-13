<template>
  <div class="page">
    <RouterLink :to="{ name: 'class', params: { id: classId } }" class="small">← {{ hw?.classTitle ?? 'Lớp học' }}</RouterLink>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="hw">
      <article class="panel stack">
        <div class="row between">
          <h1 style="margin: 0">{{ hw.title }}</h1>
          <span class="row">
            <hui-tag v-if="status" :color="HOMEWORK_STATUS_COLORS[status]">{{ HOMEWORK_STATUS_LABELS[status] }}</hui-tag>
            <hui-tag v-else color="secondary">Chưa nộp</hui-tag>
          </span>
        </div>
        <p class="muted small" style="margin: 0">
          <template v-if="hw.dueAt">
            Hạn nộp <b :class="{ 'field-error': hw.overdue && !status }">{{ formatDateTime(hw.dueAt) }}</b>
            <template v-if="hw.overdue && !status"> — đã quá hạn, vẫn nộp được (ghi là nộp muộn)</template>
          </template>
          <template v-else>Không có hạn nộp</template>
          · {{ hw.maxScore != null ? `Thang điểm ${formatScore(hw.maxScore)}` : 'Giáo viên nhận xét, không chấm điểm' }}
        </p>
        <hui-alert v-if="hw.isPreview" severity="info" description="Bạn đang xem thử như học sinh — giáo viên không nộp bài được." />
        <hui-markdown v-if="hw.instructions" :value="hw.instructions" />
      </article>

      <section v-if="sub" class="panel stack">
        <div class="row between">
          <h2 style="margin: 0">Bài đã nộp</h2>
          <span class="muted small">
            Lần {{ sub.submitCount }} · {{ formatDateTime(sub.submittedAt) }}<template v-if="sub.late"> · <b class="field-error">nộp muộn</b></template>
          </span>
        </div>
        <div v-if="sub.status !== 'submitted'" class="result" :class="sub.status">
          <div v-if="sub.status === 'graded'" class="score">
            <template v-if="sub.score != null">{{ formatScore(sub.score) }}<small> / {{ formatScore(sub.maxScore) }}</small></template>
            <template v-else>Đã nhận xét</template>
          </div>
          <div v-else class="small"><b>Giáo viên trả lại để làm lại</b> — sửa bài rồi nộp lại bên dưới.</div>
          <hui-markdown v-if="sub.feedback" :value="sub.feedback" />
        </div>
        <FileThumbs :files="sub.files" />
        <p v-if="sub.note" class="small" style="margin: 0"><b>Lời nhắn của bạn:</b> {{ sub.note }}</p>
      </section>

      <section v-if="hw.canSubmit" class="panel stack">
        <h2 style="margin: 0">{{ sub ? 'Nộp lại' : 'Nộp bài' }}</h2>
        <p class="muted small" style="margin: 0">
          Chụp từng trang bài làm (rõ chữ, đủ sáng) hoặc gửi file PDF. Tối đa {{ MAX_FILES }} file, mỗi file 15 MB.
          <template v-if="sub">Nộp lại sẽ thay toàn bộ bài cũ — bỏ ảnh nào không dùng nữa.</template>
        </p>
        <FileThumbs :files="chosen" removable @remove="chosen.splice($event, 1)" />
        <div class="row">
          <hui-button variant="outline" :disabled="busy || chosen.length >= MAX_FILES" @hui-click="picker?.click()">Chọn ảnh / PDF</hui-button>
          <hui-button variant="outline" class="camera" :disabled="busy || chosen.length >= MAX_FILES" @hui-click="camera?.click()">Chụp ảnh</hui-button>
          <input ref="picker" type="file" multiple accept="image/png,image/jpeg,image/gif,image/webp,application/pdf" hidden @change="onFiles" />
          <input ref="camera" type="file" accept="image/*" capture="environment" hidden @change="onFiles" />
        </div>
        <label class="stack small">
          Lời nhắn cho giáo viên (không bắt buộc)
          <textarea v-model="note" class="native-input" rows="2" maxlength="2000" placeholder="Ví dụ: Câu 3 em chưa làm được" />
        </label>
        <div class="row">
          <hui-button :loading="saving" :disabled="busy || !readyIds.length" @hui-click="submit">{{ sub ? 'Nộp lại' : 'Nộp bài' }}</hui-button>
          <span v-if="busy" class="muted small">Đang tải file lên…</span>
        </div>
      </section>
    </template>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { learnApi } from '../../api';
import FileThumbs from '../../components/learning/FileThumbs.vue';
import { formatDateTime } from '../../utils/form';
import { HOMEWORK_STATUS_COLORS, HOMEWORK_STATUS_LABELS, formatScore } from '../../utils/teaching';
import { checkFile, uploadFile } from '../../utils/upload';

const MAX_FILES = 10;
const route = useRoute();
const classId = route.params.id;
const stepId = route.params.stepId;
const hw = ref(null);
const error = ref('');
const note = ref('');
/** File đã chọn: FileView (đã upload xong) hoặc { name, progress } đang upload */
const chosen = ref([]);
const saving = ref(false);
const picker = ref(null);
const camera = ref(null);

const sub = computed(() => hw.value?.submission ?? null);
const status = computed(() => sub.value?.status ?? null);
const busy = computed(() => chosen.value.some((f) => !f.id));
const readyIds = computed(() => chosen.value.filter((f) => f.id).map((f) => f.id));

function apply(view) {
  hw.value = view;
  note.value = view.submission?.note ?? '';
  // Nộp lại: bắt đầu từ các file đã nộp (bỏ / thêm tùy ý).
  chosen.value = (view.submission?.files ?? []).map((f) => f.file);
}

async function load() {
  error.value = '';
  try {
    apply(await learnApi.stepHomework(classId, stepId));
  } catch (e) {
    error.value = e.message;
  }
}

async function onFiles(e) {
  const files = [...e.target.files].slice(0, MAX_FILES - chosen.value.length);
  e.target.value = '';
  for (const file of files) {
    const problem = checkFile(file, 'homework');
    if (problem && !file.type.startsWith('image/')) {
      message.error(`${file.name}: ${problem}`);
      continue;
    }
    // Ảnh quá lớn vẫn thử (được thu nhỏ trước khi gửi); loại file sai thì báo ngay.
    chosen.value.push({ name: file.name, isImage: false, url: '#', progress: 0 });
    const entry = chosen.value[chosen.value.length - 1];
    try {
      const saved = await uploadFile(file, 'homework', { onProgress: (p) => (entry.progress = p) });
      const idx = chosen.value.indexOf(entry);
      if (idx >= 0) chosen.value.splice(idx, 1, saved);
    } catch (err) {
      chosen.value.splice(chosen.value.indexOf(entry), 1);
      message.error(`${file.name}: ${err.message}`);
    }
  }
}

async function submit() {
  saving.value = true;
  try {
    apply(await learnApi.submitHomework(classId, stepId, { note: note.value.trim(), fileIds: readyIds.value }));
    message.success('Đã nộp bài. Giáo viên sẽ chấm và nhận xét.');
  } catch (e) {
    message.error(e.fields ? Object.values(e.fields)[0][0] : e.message);
  } finally {
    saving.value = false;
  }
}

onMounted(load);
</script>

<style scoped>
.result {
  display: grid;
  gap: 6px;
  padding: 12px;
  border-radius: var(--hui-radius-md, 8px);
  background: color-mix(in srgb, var(--hui-success, #10b981) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--hui-success, #10b981) 40%, transparent);
}
.result.returned {
  background: color-mix(in srgb, var(--hui-warning, #d97706) 8%, transparent);
  border-color: color-mix(in srgb, var(--hui-warning, #d97706) 40%, transparent);
}
.score {
  font-size: 1.8rem;
  font-weight: 700;
  color: var(--hui-success, #059669);
}
.score small {
  font-size: 1rem;
  color: var(--hui-text-secondary, #6b7280);
}
textarea.native-input {
  font: inherit;
  resize: vertical;
}
/* Nút chụp ảnh chỉ có ý nghĩa trên điện thoại / máy tính bảng */
@media (hover: hover) and (pointer: fine) {
  .camera {
    display: none;
  }
}
</style>

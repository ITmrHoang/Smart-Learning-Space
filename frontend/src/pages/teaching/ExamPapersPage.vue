<template>
  <div class="page">
    <div class="page-header no-print">
      <div class="stack" style="gap: 4px">
        <RouterLink :to="{ name: 'teach-exam', params: { id: route.params.id } }" class="small">← Về đề</RouterLink>
        <h1>Trộn mã đề · {{ data?.title }}</h1>
        <span v-if="data?.papers.length" class="muted small">
          {{ data.papers.length }} mã đề · {{ formatScore(data.maxScore) }} điểm · tạo lúc {{ formatDateTime(data.createdAt) }}
        </span>
      </div>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" class="no-print" />

    <section class="panel form-grid no-print">
      <div class="grid-3">
        <hui-input-number label="Số mã đề" :min="1" :max="24" :value="form.count" @change="form.count = Number($event.detail.value) || 1" />
        <hui-input-number label="Mã đầu tiên" :min="1" :max="9900" :value="form.firstCode" @change="form.firstCode = Number($event.detail.value) || 101" />
        <div class="field">
          <span>&nbsp;</span>
          <hui-button :loading="generating" @hui-click="generate">{{ data?.papers.length ? 'Tạo lại bộ mã đề' : 'Tạo mã đề' }}</hui-button>
        </div>
      </div>
      <p class="muted small" style="margin: 0">
        Mỗi mã rút câu theo ma trận và trộn câu trong từng phần, trộn phương án theo cài đặt của đề. Bộ mã đề được lưu lại để in /
        tải lại đúng như cũ (và để chấm phiếu tô sau này).
      </p>
      <div class="grid-3">
        <hui-input v-model="header.school" label="Đơn vị (trường / trung tâm)" placeholder="Trung tâm …" />
        <hui-input v-model="header.examName" label="Tên kỳ kiểm tra" placeholder="ĐỀ KIỂM TRA GIỮA KỲ I" />
        <hui-input-number label="Thời gian (phút)" :min="1" :max="600" :value="header.minutes" @change="header.minutes = $event.detail.value || null" />
      </div>
    </section>

    <section v-if="data?.papers.length" class="panel stack no-print">
      <div class="row between">
        <hui-segmented :options="viewOptions" :value="view" @change="view = $event.detail.value" />
        <div class="row">
          <hui-button variant="outline" :loading="exporting === 'papers'" @hui-click="download('papers')">Tải Word các mã đề</hui-button>
          <hui-button variant="outline" :loading="exporting === 'key'" @hui-click="download('key')">Tải Word đáp án</hui-button>
          <hui-button @hui-click="print">In / Lưu PDF</hui-button>
        </div>
      </div>
      <hui-alert
        v-if="!shuffled"
        severity="warning"
        description="Đề đang tắt trộn câu và trộn phương án — các mã đề sẽ giống nhau (trừ câu rút theo ma trận). Bật trộn ở trang soạn đề rồi tạo lại."
      />
    </section>

    <article v-show="data?.papers.length" ref="sheet" class="sheet" />
  </div>
</template>

<script setup>
/** Trộn N mã đề từ một đề, xem / in / tải Word các mã đề và bảng đáp án (M40). */
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, nextTick, onMounted, reactive, ref, watch } from 'vue';
import { useRoute } from 'vue-router';

import { teachingApi } from '../../api';
import { downloadBlob, exportDocx } from '../../utils/docx/document';
import { renderAllPapersDom, renderAnswerKeyDom, renderPaperDom } from '../../utils/examPaperRender';
import { formatDateTime } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { formatScore } from '../../utils/teaching';

const HEADER_KEY = 'sls.paperHeader';

const route = useRoute();
const { data: master, load: loadMaster } = useMasterData();
const data = ref(null);
const exam = ref(null);
const error = ref('');
const generating = ref(false);
const exporting = ref('');
const sheet = ref(null);
const view = ref('all');
const form = reactive({ count: 4, firstCode: 101 });
const header = reactive({ school: '', examName: '', minutes: null });

const subjectName = computed(() => master.value?.subjects.find((s) => s.id === data.value?.subjectId)?.name ?? '');
const shuffled = computed(() => !exam.value || exam.value.shuffleQuestions || exam.value.shuffleOptions);
const viewOptions = computed(() => [
  ...(data.value?.papers ?? []).map((p) => ({ value: p.code, label: `Mã ${p.code}` })),
  { value: 'all', label: 'Tất cả' },
  { value: 'key', label: 'Đáp án' },
]);
const headerInfo = () => ({ ...header, subjectName: subjectName.value, title: data.value?.title });

function rememberHeader() {
  try {
    localStorage.setItem(HEADER_KEY, JSON.stringify({ school: header.school }));
  } catch {
    /* trình duyệt chặn lưu: bỏ qua */
  }
}

function render() {
  const el = sheet.value;
  if (!el || !data.value?.papers.length) return;
  const papers = data.value.papers;
  let node;
  if (view.value === 'key') node = renderAnswerKeyDom(papers, headerInfo());
  else if (view.value === 'all') node = renderAllPapersDom(papers, headerInfo());
  else node = renderPaperDom(papers.find((p) => p.code === view.value) ?? papers[0], headerInfo());
  el.replaceChildren(node);
}

async function load() {
  error.value = '';
  try {
    await loadMaster();
    const [papers, e] = await Promise.all([teachingApi.examPapers(route.params.id), teachingApi.exam(route.params.id)]);
    data.value = papers;
    exam.value = e;
    header.examName ||= e.title;
    header.minutes ??= e.timeLimitMinutes;
    await nextTick();
    render();
  } catch (e) {
    error.value = e.message;
  }
}

async function generate() {
  const n = data.value?.papers.length ?? 0;
  if (n && !window.confirm(`Tạo lại sẽ thay ${n} mã đề cũ (đã in thì đáp án cũ không còn đúng). Tiếp tục?`)) return;
  generating.value = true;
  try {
    data.value = await teachingApi.generatePapers(route.params.id, { count: form.count, firstCode: form.firstCode });
    view.value = 'all';
    message.success(`Đã tạo ${data.value.papers.length} mã đề.`);
    await nextTick();
    render();
  } catch (e) {
    message.error(e.message);
  } finally {
    generating.value = false;
  }
}

function fileSlug(text) {
  return (
    String(text || 'de')
      .normalize('NFD')
      .replace(/[̀-ͯ]/g, '')
      .replace(/đ/g, 'd')
      .replace(/Đ/g, 'D')
      .replace(/[^a-zA-Z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .toLowerCase()
      .slice(0, 60) || 'de'
  );
}

async function download(kind) {
  exporting.value = kind;
  rememberHeader();
  try {
    const papers = data.value.papers;
    const root = kind === 'key' ? renderAnswerKeyDom(papers, headerInfo()) : renderAllPapersDom(papers, headerInfo());
    const title = `${data.value.title} — ${kind === 'key' ? 'đáp án' : 'các mã đề'}`;
    const blob = await exportDocx({ title, root });
    downloadBlob(blob, `${fileSlug(data.value.title)}-${kind === 'key' ? 'dap-an' : 'ma-de'}.docx`);
  } catch (e) {
    message.error(`Không tạo được file Word: ${e.message}`);
  } finally {
    exporting.value = '';
  }
}

function print() {
  rememberHeader();
  window.print();
}

watch([view, () => ({ ...header }), subjectName], render, { deep: true });

onMounted(() => {
  try {
    const saved = JSON.parse(localStorage.getItem(HEADER_KEY) || '{}');
    if (saved.school) header.school = saved.school;
  } catch {
    /* không đọc được: để trống */
  }
  load();
});
</script>

<style scoped>
.sheet {
  max-width: 210mm;
  margin: 0 auto;
  padding: 20mm 20mm 20mm 25mm;
  background: #fff;
  color: #000;
  font-family: 'Times New Roman', Times, serif;
  font-size: 13pt;
  line-height: 1.4;
  box-shadow: 0 2px 12px rgb(0 0 0 / 12%);
}
.sheet :deep(table.plain) {
  width: 100%;
  border-collapse: collapse;
}
.sheet :deep(table.plain td) {
  vertical-align: top;
  padding: 2px 6px;
}
.sheet :deep(table.options td) {
  width: 25%;
}
.sheet :deep(table.key) {
  border-collapse: collapse;
  margin: 6px 0 12px;
}
.sheet :deep(table.key th),
.sheet :deep(table.key td) {
  border: 1px solid #000;
  padding: 3px 10px;
  text-align: center;
}
.sheet :deep(table.rubric td:first-child) {
  text-align: left;
}
.sheet :deep(.center) {
  text-align: center;
}
.sheet :deep(p) {
  margin: 4px 0;
}
.sheet :deep(h1) {
  font-size: 15pt;
  text-align: center;
}
.sheet :deep(h2) {
  font-size: 13pt;
  margin: 12px 0 4px;
}
.sheet :deep(.page-break) {
  break-after: page;
  border-top: 1px dashed #bbb;
  margin: 18px 0;
}
@media print {
  @page {
    size: A4;
    margin: 15mm 15mm 15mm 20mm;
  }
  :global(body *) {
    visibility: hidden;
  }
  .sheet,
  .sheet :deep(*) {
    visibility: visible;
  }
  .sheet {
    position: absolute;
    left: 0;
    top: 0;
    max-width: none;
    padding: 0;
    box-shadow: none;
  }
  .sheet :deep(.page-break) {
    border: 0;
    margin: 0;
  }
}
</style>

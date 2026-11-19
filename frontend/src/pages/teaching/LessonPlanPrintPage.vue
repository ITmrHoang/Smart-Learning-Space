<template>
  <div class="print-page">
    <div class="toolbar no-print">
      <RouterLink :to="{ name: 'teach-plan', params: { id: route.params.id } }" class="small">← Quay lại soạn</RouterLink>
      <span class="spacer" />
      <span class="muted small">Khổ A4 · lề trái 3 cm · Times New Roman 13. Chọn "Lưu dưới dạng PDF" trong hộp thoại in để có file PDF.</span>
      <hui-button variant="outline" :loading="exporting" :disabled="!plan" @hui-click="exportWord">Tải Word</hui-button>
      <hui-button :disabled="!plan" @hui-click="print">In / Lưu PDF</hui-button>
    </div>
    <hui-alert v-if="error" class="no-print" severity="error" :description="error" />
    <article ref="sheet" class="sheet" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { useRoute } from 'vue-router';

import { teachingApi } from '../../api';
import { downloadBlob, exportDocx } from '../../utils/docx/document';
import { docxFileName } from '../../utils/lessonPlan';
import { loadPlanQuestions, renderPlanDom } from '../../utils/lessonPlanRender';
import { useMasterData } from '../../utils/masterData';

const route = useRoute();
const { data: master, load: loadMaster } = useMasterData();
const sheet = ref(null);
const plan = ref(null);
const error = ref('');
const exporting = ref(false);
const previousTitle = document.title;
let questions = new Map();

function names() {
  return {
    subjectName: master.value?.subjects.find((s) => s.id === plan.value.subjectId)?.name ?? '',
    gradeName: master.value?.grades.find((g) => g.id === plan.value.gradeId)?.name ?? '',
  };
}

function print() {
  window.print();
}

async function exportWord() {
  exporting.value = true;
  try {
    const root = renderPlanDom(plan.value, { ...names(), questions });
    downloadBlob(await exportDocx({ title: plan.value.title, root }), docxFileName(plan.value.title));
  } catch (e) {
    message.error(`Không xuất được file Word: ${e.message}`);
  } finally {
    exporting.value = false;
  }
}

onMounted(async () => {
  try {
    await loadMaster();
    plan.value = await teachingApi.lessonPlan(route.params.id);
    questions = await loadPlanQuestions(plan.value, teachingApi.question);
    sheet.value.replaceChildren(renderPlanDom(plan.value, { ...names(), questions }));
    // Tên file PDF khi "Lưu dưới dạng PDF" lấy theo tiêu đề trang
    document.title = docxFileName(plan.value.title).replace(/\.docx$/, '');
  } catch (e) {
    error.value = e.message;
  }
});

onBeforeUnmount(() => {
  document.title = previousTitle;
});
</script>

<style scoped>
.print-page {
  min-height: 100vh;
  background: #e5e7eb;
  padding: 0 0 32px;
}
.toolbar {
  position: sticky;
  top: 0;
  z-index: 1;
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
  padding: 10px 16px;
  background: #fff;
  border-bottom: 1px solid #d1d5db;
}
.spacer {
  flex: 1;
}
.sheet {
  box-sizing: border-box;
  width: 210mm;
  max-width: calc(100% - 32px);
  min-height: 297mm;
  margin: 24px auto;
  padding: 20mm 20mm 20mm 30mm;
  background: #fff;
  color: #000;
  box-shadow: 0 2px 12px rgb(0 0 0 / 15%);
  font: 13pt/1.45 'Times New Roman', Times, serif;
  text-align: justify;
}
.sheet :deep(.plan-head) {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: 6px;
}
.sheet :deep(.plan-head td) {
  vertical-align: top;
  width: 50%;
}
.sheet :deep(.plan-head p),
.sheet :deep(.plan-dates) {
  margin: 0;
}
.sheet :deep(.plan-title) {
  margin: 16px 0 4px;
  font-size: 14pt;
  text-align: center;
}
.sheet :deep(.plan-sub) {
  margin: 0;
  text-align: center;
}
.sheet :deep(h2) {
  margin: 14px 0 6px;
  font-size: 13pt;
}
.sheet :deep(h3) {
  margin: 12px 0 4px;
  font-size: 13pt;
}
.sheet :deep(p) {
  margin: 0 0 4px;
}
.sheet :deep(.option) {
  margin-left: 24px;
}
.sheet :deep(table.plan-table) {
  width: 100%;
  border-collapse: collapse;
}
.sheet :deep(.plan-table th),
.sheet :deep(.plan-table td),
.sheet :deep(.md-table td),
.sheet :deep(.md-table th) {
  border: 1px solid #000;
  padding: 4px 6px;
  vertical-align: top;
}
.sheet :deep(img) {
  max-width: 100%;
}
.sheet :deep(hui-plot) {
  display: block;
  max-width: 560px;
  margin: 8px auto;
  --hui-plot-height: 300px;
}

@media print {
  /* Lề đặt ở @page để trang nào cũng có lề (không chỉ trang đầu) */
  @page {
    size: A4;
    margin: 20mm 20mm 20mm 30mm;
  }
  .no-print {
    display: none !important;
  }
  .print-page {
    background: none;
    padding: 0;
  }
  .sheet {
    width: auto;
    max-width: none;
    min-height: 0;
    margin: 0;
    padding: 0;
    box-shadow: none;
  }
}
</style>

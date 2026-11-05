<template>
  <div class="page">
    <div class="page-header">
      <h1>Ngân hàng câu hỏi</h1>
      <div class="row">
        <hui-button variant="outline" @hui-click="importOpen = true">Nhập từ Excel</hui-button>
        <hui-button @hui-click="openEditor(null)">Thêm câu hỏi</hui-button>
      </div>
    </div>

    <div class="layout">
      <aside class="panel folders">
        <div class="row between">
          <b>Thư mục</b>
          <hui-button size="sm" variant="text" @hui-click="newFolder(null)">+ Thư mục</hui-button>
        </div>
        <ul class="clean-list">
          <li><button class="folder" :class="{ active: folderFilter === '' }" @click="setFolder('')">Tất cả</button></li>
          <li><button class="folder" :class="{ active: folderFilter === 'none' }" @click="setFolder('none')">Chưa xếp thư mục</button></li>
          <li v-for="f in flatFolders" :key="f.id" class="folder-row" :style="{ paddingLeft: `${f.depth * 14}px` }">
            <button class="folder" :class="{ active: folderFilter === f.id }" @click="setFolder(f.id)">
              {{ f.name }} <span class="muted small">({{ f.questionCount }})</span>
            </button>
            <span class="folder-actions">
              <button title="Thêm thư mục con" @click="newFolder(f.id)">+</button>
              <button title="Đổi tên" @click="renameFolder(f)">✎</button>
              <button title="Xóa" @click="removeFolder(f)">✕</button>
            </span>
          </li>
        </ul>
      </aside>

      <section class="stack">
        <div class="panel toolbar">
          <hui-input class="grow" type="search" label="Tìm trong nội dung" placeholder="Gõ không dấu cũng được" clearable @hui-search="list.setFilter('q', $event.detail.value)" @clear="list.setFilter('q', '')" />
          <hui-select label="Môn" allow-clear :options="subjectOptions" placeholder="Tất cả" @change="onSubjectFilter($event.detail.value)" />
          <hui-select label="Danh mục" allow-clear :options="tagOptions" placeholder="Tất cả" :value="list.filters.tagId" @change="list.setFilter('tagId', $event.detail.value)" />
          <hui-select label="Dạng" allow-clear :options="kindOptions" placeholder="Tất cả" @change="list.setFilter('kind', $event.detail.value)" />
          <hui-select label="Mức độ" allow-clear :options="levelOptions" placeholder="Tất cả" @change="list.setFilter('level', $event.detail.value)" />
        </div>

        <hui-alert v-if="list.error.value" severity="error" :description="list.error.value" />
        <p class="muted small">{{ list.total.value }} câu hỏi</p>
        <div v-if="!list.loading.value && !list.items.value.length" class="panel empty muted">Chưa có câu hỏi nào.</div>
        <article v-for="q in list.items.value" :key="q.id" class="panel q-card">
          <div class="row between">
            <div class="row">
              <hui-tag size="sm">{{ QUESTION_KIND_LABELS[q.kind] }}</hui-tag>
              <hui-tag size="sm" :color="LEVEL_COLORS[q.level]">{{ LEVEL_LABELS[q.level] }}</hui-tag>
              <span class="muted small">{{ subjectName(q.subjectId) }} · phiên bản {{ q.version }} · {{ q.usedCount }} lượt làm</span>
            </div>
            <div class="row">
              <hui-button size="sm" variant="text" @hui-click="toggle(q.id)">{{ expanded.has(q.id) ? 'Thu gọn' : 'Xem đáp án' }}</hui-button>
              <hui-button size="sm" variant="outline" @hui-click="openEditor(q)">Sửa</hui-button>
              <hui-button size="sm" variant="text" @hui-click="archive(q)">Ẩn</hui-button>
            </div>
          </div>
          <hui-markdown :value="q.content" />
          <QuestionView v-if="expanded.has(q.id)" :item="asItem(q)" :number="1" readonly bare />
        </article>
        <hui-pagination
          class="pager"
          :total="list.total.value"
          :current="list.page.value"
          :pageSize="list.pageSize.value"
          show-total
          hide-on-single-page
          @change="list.onPage($event.detail)"
        />
      </section>
    </div>

    <QuestionEditorModal
      v-model:open="editorOpen"
      :question="editing"
      :defaultSubjectId="list.filters.subjectId || null"
      :defaultFolderId="folderFilter && folderFilter !== 'none' ? folderFilter : null"
      :folders="flatFolders"
      @saved="onSaved"
    />
    <ImportQuestionsModal
      v-model:open="importOpen"
      :defaultSubjectId="list.filters.subjectId || null"
      :defaultFolderId="folderFilter && folderFilter !== 'none' ? folderFilter : null"
      :folders="flatFolders"
      @saved="reload"
    />
    <PromptModal v-model:open="prompt.open" :title="prompt.title" label="Tên thư mục" :initial="prompt.initial" :saveFn="prompt.save" @saved="loadFolders" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, reactive, ref } from 'vue';

import { catalogApi, teachingApi } from '../../api';
import PromptModal from '../../components/PromptModal.vue';
import ImportQuestionsModal from '../../components/teaching/ImportQuestionsModal.vue';
import QuestionEditorModal from '../../components/teaching/QuestionEditorModal.vue';
import QuestionView from '../../components/teaching/QuestionView.vue';
import { flattenFolders } from '../../utils/folders';
import { usePagedList } from '../../utils/list';
import { useMasterData } from '../../utils/masterData';
import { LEVEL_COLORS, LEVEL_LABELS, QUESTION_KIND_LABELS, toOptions } from '../../utils/teaching';

const { data: master, load: loadMaster } = useMasterData();
const list = usePagedList((q) => teachingApi.questions(q), { q: '', subjectId: '', folderId: '', tagId: '', kind: '', level: '' });
const folders = ref([]);
const flatFolders = computed(() => flattenFolders(folders.value));
const folderFilter = ref('');
const tags = ref([]);
const expanded = reactive(new Set());
const editorOpen = ref(false);
const editing = ref(null);
const importOpen = ref(false);
const prompt = reactive({ open: false, title: '', initial: '', save: async () => {} });

const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));
const tagOptions = computed(() => tags.value.map((t) => ({ value: t.id, label: t.name })));
const kindOptions = toOptions(QUESTION_KIND_LABELS);
const levelOptions = toOptions(LEVEL_LABELS);
const subjectName = (id) => master.value?.subjects.find((s) => s.id === id)?.name ?? '';

/** Câu hỏi của giáo viên → dạng câu trong bài làm (để xem đáp án bằng QuestionView). */
function asItem(q) {
  return {
    position: 0,
    kind: q.kind,
    content: '',
    points: 1,
    options: q.options,
    answer: null,
    checked: true,
    result: {
      isCorrect: true,
      score: 1,
      correct: q.options.filter((o) => o.correct).map((o) => o.id),
      accepted: q.accepted,
      explanation: q.explanation,
    },
  };
}

async function onSubjectFilter(id) {
  list.filters.tagId = '';
  tags.value = id ? await catalogApi.tags(id).catch(() => []) : [];
  list.setFilter('subjectId', id);
}

function setFolder(id) {
  folderFilter.value = id;
  list.setFilter('folderId', id);
}

function toggle(id) {
  if (expanded.has(id)) expanded.delete(id);
  else expanded.add(id);
}

function openEditor(q) {
  editing.value = q;
  editorOpen.value = true;
}

async function loadFolders() {
  folders.value = await teachingApi.folders().catch(() => []);
}

function reload() {
  list.load();
  loadFolders();
}

function onSaved() {
  message.success('Đã lưu câu hỏi.');
  reload();
}

async function archive(q) {
  if (!window.confirm('Ẩn câu hỏi này khỏi ngân hàng? Đề đang dùng cố định câu này vẫn giữ, ma trận không rút câu này nữa.')) return;
  try {
    await teachingApi.archiveQuestion(q.id);
    message.success('Đã ẩn câu hỏi.');
    reload();
  } catch (e) {
    message.error(e.message);
  }
}

function newFolder(parentId) {
  Object.assign(prompt, {
    open: true,
    title: parentId ? 'Thêm thư mục con' : 'Thêm thư mục',
    initial: '',
    save: (name) => teachingApi.createFolder({ name, parentId }),
  });
}

function renameFolder(f) {
  Object.assign(prompt, {
    open: true,
    title: 'Đổi tên thư mục',
    initial: f.name,
    save: (name) => teachingApi.updateFolder(f.id, { name, parentId: f.parentId }),
  });
}

async function removeFolder(f) {
  if (!window.confirm(`Xóa thư mục "${f.name}" và các thư mục con? Câu hỏi bên trong chuyển về "Chưa xếp thư mục".`)) return;
  try {
    await teachingApi.deleteFolder(f.id);
    if (folderFilter.value === f.id) setFolder('');
    reload();
  } catch (e) {
    message.error(e.message);
  }
}

onMounted(() => {
  loadMaster();
  loadFolders();
  list.load();
});
</script>

<style scoped>
.layout {
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}
.folders {
  position: sticky;
  top: 72px;
  display: grid;
  gap: 8px;
}
.folder {
  border: 0;
  background: none;
  font: inherit;
  cursor: pointer;
  text-align: left;
  padding: 4px 8px;
  border-radius: 6px;
  color: inherit;
  flex: 1;
}
.folder.active {
  background: color-mix(in srgb, var(--hui-primary, #1677ff) 12%, transparent);
  color: var(--hui-primary, #1677ff);
}
.folder-row {
  display: flex;
  align-items: center;
}
.folder-actions {
  display: none;
  gap: 2px;
}
.folder-row:hover .folder-actions {
  display: inline-flex;
}
.folder-actions button {
  border: 0;
  background: none;
  cursor: pointer;
  color: var(--hui-text-secondary, #6b7280);
}
.q-card {
  display: grid;
  gap: 10px;
}
.pager {
  display: flex;
  justify-content: flex-end;
}
@media (max-width: 860px) {
  .layout {
    grid-template-columns: 1fr;
  }
  .folders {
    position: static;
  }
}
</style>

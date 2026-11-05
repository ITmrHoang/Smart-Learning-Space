<template>
  <div class="page">
    <div class="page-header">
      <div class="stack" style="gap: 4px">
        <RouterLink to="/day-hoc/lop-hoc" class="small">← Lớp học</RouterLink>
        <h1>{{ cls?.title ?? 'Lớp học' }}</h1>
      </div>
      <div v-if="cls" class="row">
        <hui-tag v-if="cls.archived" color="danger">Đã lưu trữ</hui-tag>
        <RouterLink :to="{ name: 'class', params: { id: cls.id } }">Xem như học sinh →</RouterLink>
      </div>
    </div>
    <hui-alert v-if="error" severity="error" :description="error" />

    <template v-if="cls">
      <hui-segmented :options="tabs" :value="tab" @change="tab = $event.detail.value" />

      <section v-if="tab === 'path'" class="panel">
        <PathEditor :classId="cls.id" @published="reload" />
      </section>

      <section v-else-if="tab === 'members'" class="stack">
        <div class="panel stack">
          <h2>Mời học sinh</h2>
          <div class="row">
            <span>Mã mời: <b class="code">{{ cls.inviteCode }}</b></span>
            <hui-copy :value="cls.inviteCode" size="sm" />
            <span class="muted small">Link lớp học: </span>
            <code class="small">{{ classLink }}</code>
            <hui-copy :value="classLink" size="sm" />
            <hui-button size="sm" variant="text" @hui-click="regenerate">Đổi mã mời</hui-button>
          </div>
          <form class="row" @submit.prevent="addMember">
            <hui-input v-model="newLogin" class="grow" label="Thêm học sinh bằng username hoặc email" :error="memberError" />
            <hui-button type="submit" :loading="adding">Thêm</hui-button>
          </form>
        </div>

        <div v-if="pending.length" class="panel stack">
          <h2>Chờ duyệt ({{ pending.length }})</h2>
          <ul class="clean-list">
            <li v-for="m in pending" :key="m.userId" class="row between member">
              <span>{{ m.fullName }} <span class="muted small">@{{ m.username }} · {{ m.email }}</span></span>
              <span class="row">
                <hui-button size="sm" @hui-click="change(m, 'approve')">Duyệt</hui-button>
                <hui-button size="sm" variant="outline" @hui-click="change(m, 'reject')">Từ chối</hui-button>
              </span>
            </li>
          </ul>
        </div>

        <div class="panel stack">
          <h2>Học sinh ({{ active.length }})</h2>
          <p v-if="!active.length" class="muted small">Chưa có học sinh nào.</p>
          <ul class="clean-list">
            <li v-for="m in active" :key="m.userId" class="row between member">
              <span>{{ m.fullName }} <span class="muted small">@{{ m.username }} · vào lớp {{ formatDateTime(m.joinedAt) }}</span></span>
              <hui-button size="sm" variant="text" @hui-click="change(m, 'remove')">Mời ra</hui-button>
            </li>
          </ul>
        </div>
      </section>

      <section v-else-if="tab === 'progress'" class="panel stack">
        <div class="row between">
          <h2 style="margin: 0">Tiến độ học sinh</h2>
          <hui-button size="sm" variant="text" @hui-click="loadProgress">Tải lại</hui-button>
        </div>
        <p v-if="!progress?.students.length" class="muted small">Chưa có học sinh hoặc lộ trình chưa xuất bản.</p>
        <div v-else class="table-wrap">
          <table class="progress">
            <thead>
              <tr>
                <th>Học sinh</th>
                <th>Hoàn thành</th>
                <th v-for="(s, i) in progress.steps" :key="s.id" :title="s.title">{{ i + 1 }}. {{ s.title }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="st in progress.students" :key="st.userId">
                <td>{{ st.fullName }}<br /><span class="muted small">@{{ st.username }}</span></td>
                <td>
                  {{ st.progress.percent }}%
                  <div class="progress-bar"><span :style="{ width: `${st.progress.percent}%` }" /></div>
                </td>
                <td v-for="c in st.cells" :key="c.stepId" class="cell" :class="{ done: c.done }">
                  <template v-if="c.score != null">{{ formatScore(c.score) }}/{{ formatScore(c.maxScore) }}</template>
                  <template v-else>{{ c.done ? '✓' : '' }}</template>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>

      <section v-else class="panel stack">
        <dl class="info">
          <dt>Môn</dt>
          <dd>{{ subjectName }}</dd>
          <dt>Hình thức</dt>
          <dd>{{ MODE_LABELS[cls.mode] }}</dd>
          <dt>Ai thấy</dt>
          <dd>{{ VISIBILITY_LABELS[cls.visibility] }}</dd>
          <dt>Ghi danh</dt>
          <dd>{{ ENROLLMENT_LABELS[cls.enrollment] }}</dd>
          <dt>Tạo lúc</dt>
          <dd>{{ formatDateTime(cls.createdAt) }}</dd>
        </dl>
        <hui-markdown v-if="cls.description" :value="cls.description" />
        <div class="row">
          <hui-button variant="outline" @hui-click="editOpen = true">Sửa thông tin</hui-button>
          <hui-button variant="outline" @hui-click="copyOpen = true">Tạo lớp học mới từ lớp này</hui-button>
          <hui-button v-if="!cls.archived" variant="danger" @hui-click="archive(true)">Lưu trữ lớp học</hui-button>
          <hui-button v-else variant="outline" @hui-click="archive(false)">Khôi phục</hui-button>
        </div>
      </section>
    </template>

    <ClassFormModal v-model:open="editOpen" :cls="cls" @saved="onSaved" />
    <PromptModal
      v-model:open="copyOpen"
      title="Tạo lớp học mới từ lớp này"
      label="Tên lớp học mới"
      :initial="cls ? `${cls.title} (bản sao)` : ''"
      :saveFn="(title) => teachingApi.copyClass(cls.id, title)"
      @saved="onCopied"
    />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { teachingApi } from '../../api';
import PromptModal from '../../components/PromptModal.vue';
import ClassFormModal from '../../components/teaching/ClassFormModal.vue';
import PathEditor from '../../components/teaching/PathEditor.vue';
import { formatDateTime } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { ENROLLMENT_LABELS, MODE_LABELS, VISIBILITY_LABELS, formatScore } from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const cls = ref(null);
const members = ref([]);
const progress = ref(null);
const error = ref('');
const tab = ref('path');
const editOpen = ref(false);
const copyOpen = ref(false);
const newLogin = ref('');
const memberError = ref('');
const adding = ref(false);

const tabs = computed(() => [
  { value: 'path', label: 'Lộ trình' },
  { value: 'members', label: `Học sinh${cls.value?.pendingCount ? ` (${cls.value.pendingCount} chờ duyệt)` : ''}` },
  { value: 'progress', label: 'Tiến độ' },
  { value: 'info', label: 'Thông tin' },
]);
const pending = computed(() => members.value.filter((m) => m.status === 'pending'));
const active = computed(() => members.value.filter((m) => m.status === 'active'));
const subjectName = computed(() => master.value?.subjects.find((s) => s.id === cls.value?.subjectId)?.name ?? '');
const classLink = computed(() => (cls.value ? `${window.location.origin}/lop-hoc/${cls.value.id}` : ''));

async function reload() {
  try {
    cls.value = await teachingApi.class(route.params.id);
  } catch (e) {
    error.value = e.message;
  }
}

async function loadMembers() {
  members.value = await teachingApi.members(route.params.id).catch(() => []);
}

async function loadProgress() {
  progress.value = await teachingApi.progress(route.params.id).catch(() => null);
}

watch(tab, (t) => {
  if (t === 'members') loadMembers();
  if (t === 'progress') loadProgress();
});

async function addMember() {
  memberError.value = '';
  adding.value = true;
  try {
    await teachingApi.addMember(cls.value.id, newLogin.value.trim());
    message.success('Đã thêm học sinh.');
    newLogin.value = '';
    loadMembers();
    reload();
  } catch (e) {
    memberError.value = e.fields?.login?.[0] ?? e.message;
  } finally {
    adding.value = false;
  }
}

async function change(m, action) {
  const ask = { remove: `Mời ${m.fullName} ra khỏi lớp học?`, reject: `Từ chối ${m.fullName}?` }[action];
  if (ask && !window.confirm(ask)) return;
  try {
    await teachingApi.changeMember(cls.value.id, m.userId, action);
    loadMembers();
    reload();
  } catch (e) {
    message.error(e.message);
  }
}

async function regenerate() {
  if (!window.confirm('Đổi mã mời? Mã cũ sẽ không dùng được nữa.')) return;
  cls.value = await teachingApi.regenerateCode(cls.value.id);
}

async function archive(value) {
  if (value && !window.confirm('Lưu trữ lớp học? Học sinh sẽ không vào học được nữa (khôi phục được).')) return;
  cls.value = value ? await teachingApi.archiveClass(cls.value.id) : await teachingApi.restoreClass(cls.value.id);
}

function onSaved(c) {
  cls.value = c;
  message.success('Đã lưu lớp học.');
}

function onCopied(c) {
  message.success('Đã tạo lớp học mới. Lộ trình được chép ở dạng bản nháp.');
  router.push({ name: 'teach-class', params: { id: c.id } });
}

watch(
  () => route.params.id,
  (id) => {
    if (id) {
      cls.value = null;
      tab.value = 'path';
      reload();
    }
  },
);

onMounted(() => {
  loadMaster();
  reload();
});
</script>

<style scoped>
.code {
  font-family: var(--hui-font-mono, monospace);
  font-size: 1.1rem;
  letter-spacing: 2px;
}
.member {
  padding: 8px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.table-wrap {
  overflow-x: auto;
}
.progress {
  border-collapse: collapse;
  min-width: 100%;
}
.progress th,
.progress td {
  border: 1px solid var(--hui-border-secondary, #eceef1);
  padding: 6px 8px;
  text-align: left;
  font-size: 0.875rem;
  white-space: nowrap;
}
.progress th {
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.cell {
  text-align: center !important;
}
.cell.done {
  background: color-mix(in srgb, var(--hui-success, #52c41a) 14%, transparent);
}
</style>

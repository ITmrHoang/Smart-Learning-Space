<template>
  <div class="stack">
    <div class="row between">
      <h2 style="margin: 0">Hỏi đáp</h2>
      <hui-button size="sm" @hui-click="askOpen = true">Đặt câu hỏi</hui-button>
    </div>

    <!-- Giáo viên: báo vi phạm đang chờ xử lý -->
    <div v-if="teacher && reports.length" class="reports stack">
      <h3 style="margin: 0">Báo vi phạm cần xử lý ({{ reports.length }})</h3>
      <ul class="clean-list">
        <li v-for="r in reports" :key="r.id" class="report">
          <div class="stack" style="gap: 4px">
            <span class="small">
              <b>{{ r.reporterName }}</b> báo {{ r.targetType === 'thread' ? 'câu hỏi' : 'một trả lời' }} của <b>{{ r.authorName }}</b> trong
              <RouterLink :to="threadLink(r.threadId)"><hui-markdown inline :value="r.threadTitle" /></RouterLink>
            </span>
            <span class="small">Lý do: {{ r.reason }}</span>
            <blockquote class="excerpt">{{ r.excerpt }}</blockquote>
          </div>
          <span class="row">
            <hui-button size="sm" variant="danger" :loading="resolving === r.id" @hui-click="resolve(r, 'hide')">Ẩn nội dung</hui-button>
            <hui-button size="sm" variant="outline" @hui-click="resolve(r, 'dismiss')">Bỏ qua</hui-button>
          </span>
        </li>
      </ul>
    </div>

    <div class="toolbar">
      <hui-segmented :options="statusOptions" :value="status" @change="setStatus($event.detail.value)" />
      <hui-input class="grow" type="search" placeholder="Tìm câu hỏi" clearable @hui-search="list.setFilter('q', $event.detail.value)" @clear="list.setFilter('q', '')" />
    </div>
    <hui-alert v-if="list.error.value" severity="error" :description="list.error.value" />
    <p v-if="!list.loading.value && !list.items.value.length" class="muted">
      {{ status === 'all' ? 'Chưa có câu hỏi nào. Chỗ nào chưa hiểu, hãy hỏi ngay!' : 'Không có câu hỏi nào.' }}
    </p>
    <ul class="clean-list threads">
      <li v-for="t in list.items.value" :key="t.id" class="thread" :class="{ hidden: t.hidden }">
        <div class="stack" style="gap: 4px; min-width: 0">
          <RouterLink :to="threadLink(t.id)" class="title"><hui-markdown inline :value="t.title" /></RouterLink>
          <span class="muted small">
            {{ t.authorName }}<hui-tag v-if="t.byTeacher" size="sm" color="primary" class="gv">Giáo viên</hui-tag>
            · {{ ago(t.lastPostAt) }}<template v-if="t.stepTitle"> · {{ t.stepTitle }}</template>
          </span>
        </div>
        <span class="row tags">
          <hui-tag v-if="t.pinned" size="sm" color="warning">Ghim</hui-tag>
          <hui-tag v-if="t.hidden" size="sm" color="danger">Đã ẩn</hui-tag>
          <hui-tag v-if="t.answered" size="sm" color="success">Đã giải đáp</hui-tag>
          <span class="count" :title="`${t.postCount} trả lời`">💬 {{ t.postCount }}</span>
        </span>
      </li>
    </ul>
    <hui-pagination :total="list.total.value" :current="list.page.value" :pageSize="list.pageSize.value" hide-on-single-page @change="list.onPage($event.detail)" />

    <AskQuestionModal v-model:open="askOpen" :classId="classId" :steps="steps" @saved="onAsked" />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { learnApi, qaApi, teachingApi } from '../../api';
import { usePagedList } from '../../utils/list';
import AskQuestionModal from './AskQuestionModal.vue';

const props = defineProps({
  classId: { type: String, required: true },
  /** Giáo viên chủ lớp: thấy báo vi phạm + câu hỏi đã ẩn */
  teacher: { type: Boolean, default: false },
});
const emit = defineEmits(['changed']);
const router = useRouter();
const list = usePagedList((q) => qaApi.threads(props.classId, q), { status: 'all', q: '' }, { pageSize: 15 });
const status = ref('all');
const statusOptions = [
  { value: 'all', label: 'Tất cả' },
  { value: 'open', label: 'Chưa giải đáp' },
  { value: 'answered', label: 'Đã giải đáp' },
  { value: 'mine', label: 'Của tôi' },
];
const steps = ref([]);
const reports = ref([]);
const askOpen = ref(false);
const resolving = ref('');

const threadLink = (id) => ({ name: 'thread', params: { id: props.classId, threadId: id } });

function ago(value) {
  const min = Math.round((Date.now() - new Date(value).getTime()) / 60_000);
  if (min < 1) return 'vừa xong';
  if (min < 60) return `${min} phút trước`;
  const h = Math.round(min / 60);
  if (h < 24) return `${h} giờ trước`;
  return new Date(value).toLocaleDateString('vi-VN');
}

function setStatus(v) {
  status.value = v;
  list.setFilter('status', v);
}

async function loadReports() {
  if (props.teacher) reports.value = await teachingApi.contentReports(props.classId).catch(() => []);
}

async function resolve(r, action) {
  resolving.value = r.id;
  try {
    await teachingApi.resolveReport(r.id, action);
    message.success(action === 'hide' ? 'Đã ẩn nội dung.' : 'Đã bỏ qua báo cáo.');
    await Promise.all([loadReports(), list.load()]);
    emit('changed');
  } catch (e) {
    message.error(e.message);
  } finally {
    resolving.value = '';
  }
}

function onAsked(t) {
  message.success('Đã gửi câu hỏi. Giáo viên sẽ nhận được thông báo.');
  router.push(threadLink(t.id));
}

onMounted(async () => {
  list.load();
  loadReports();
  const learn = await learnApi.learn(props.classId).catch(() => null);
  steps.value = (learn?.steps ?? []).filter((s) => s.kind !== 'milestone').map((s) => ({ id: s.id, title: s.title }));
});

defineExpose({ reload: () => Promise.all([list.load(), loadReports()]) });
</script>

<style scoped>
.reports {
  padding: 12px;
  border: 1px solid color-mix(in srgb, var(--hui-danger, #be1b1b) 35%, transparent);
  border-radius: var(--hui-radius-md, 8px);
  background: color-mix(in srgb, var(--hui-danger, #be1b1b) 5%, transparent);
}
.report {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  justify-content: space-between;
  align-items: flex-start;
  padding: 8px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.report:last-child {
  border-bottom: 0;
}
.excerpt {
  margin: 0;
  padding: 4px 10px;
  border-left: 3px solid var(--hui-border, #ccc);
  color: var(--hui-text-secondary, #555);
  font-size: 0.875rem;
  white-space: pre-wrap;
}
.thread {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.thread.hidden {
  opacity: 0.6;
}
.title {
  font-weight: 600;
  overflow-wrap: anywhere;
}
.gv {
  margin-left: 4px;
}
.tags {
  flex: none;
}
.count {
  font-size: 0.875rem;
  color: var(--hui-text-secondary, #555);
}
</style>

<template>
  <div class="page">
    <RouterLink :to="backLink" class="small">← Hỏi đáp của lớp học</RouterLink>
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="t">
      <article class="panel stack">
        <div class="row between">
          <h1 style="margin: 0"><hui-markdown inline :value="t.title" /></h1>
          <span class="row">
            <hui-tag v-if="t.pinned" color="warning">Ghim</hui-tag>
            <hui-tag v-if="t.hidden" color="danger">Đã ẩn (học sinh không thấy)</hui-tag>
            <hui-tag v-if="t.answered" color="success">Đã giải đáp</hui-tag>
          </span>
        </div>
        <p class="muted small" style="margin: 0">
          {{ t.authorName }}<hui-tag v-if="t.byTeacher" size="sm" color="primary" class="gv">Giáo viên</hui-tag>
          · {{ formatDateTime(t.createdAt) }}
          <template v-if="t.stepTitle"> · về <b>{{ t.stepTitle }}</b></template>
        </p>
        <hui-markdown :value="t.body" />
        <div class="row actions">
          <hui-button v-if="t.mine" size="sm" variant="text" @hui-click="editOpen = true">Sửa</hui-button>
          <hui-button v-if="(t.mine && !t.postCount) || t.canModerate" size="sm" variant="text" @hui-click="removeThread">Xóa</hui-button>
          <template v-if="t.canModerate">
            <hui-button size="sm" variant="text" @hui-click="moderate(t.pinned ? 'unpin' : 'pin')">{{ t.pinned ? 'Bỏ ghim' : 'Ghim' }}</hui-button>
            <hui-button size="sm" variant="text" @hui-click="moderate(t.hidden ? 'unhide' : 'hide')">{{ t.hidden ? 'Hiện lại' : 'Ẩn câu hỏi' }}</hui-button>
          </template>
          <hui-button v-if="!t.mine && !t.canModerate" size="sm" variant="text" :disabled="t.reportedByMe" @hui-click="openReport('thread', t.id)">
            {{ t.reportedByMe ? 'Đã báo vi phạm' : 'Báo vi phạm' }}
          </hui-button>
        </div>
      </article>

      <section class="panel stack">
        <h2 style="margin: 0">{{ t.posts.length }} trả lời</h2>
        <p v-if="!t.posts.length" class="muted" style="margin: 0">Chưa có trả lời. Bạn biết cách giải? Hãy trả lời giúp bạn nhé.</p>
        <article v-for="p in t.posts" :key="p.id" class="post" :class="{ accepted: p.accepted, teacher: p.byTeacher, hidden: p.hidden }">
          <div class="row between">
            <span class="small">
              <b>{{ p.authorName }}</b><hui-tag v-if="p.byTeacher" size="sm" color="primary" class="gv">Giáo viên</hui-tag>
              <span class="muted"> · {{ formatDateTime(p.createdAt) }}<template v-if="p.updatedAt !== p.createdAt"> · đã sửa</template></span>
            </span>
            <hui-tag v-if="p.accepted" size="sm" color="success">✓ Câu trả lời đúng</hui-tag>
          </div>
          <p v-if="p.hidden && !p.body" class="muted small" style="margin: 0">Nội dung đã bị giáo viên ẩn.</p>
          <template v-else>
            <p v-if="p.hidden" class="small field-error" style="margin: 0">Đã ẩn — học sinh không thấy nội dung này.</p>
            <div v-if="editing === p.id" class="stack">
              <MarkdownEditor v-model="editBody" label="Sửa trả lời" :rows="5" compact />
              <div class="row">
                <hui-button size="sm" :loading="busy" @hui-click="savePost(p)">Lưu</hui-button>
                <hui-button size="sm" variant="text" @hui-click="editing = ''">Hủy</hui-button>
              </div>
            </div>
            <hui-markdown v-else :value="p.body" />
          </template>
          <div class="row actions">
            <hui-button v-if="t.canAccept && !p.hidden" size="sm" variant="text" @hui-click="postAction(p, p.accepted ? 'unaccept' : 'accept')">
              {{ p.accepted ? 'Bỏ chọn đúng' : 'Chọn là câu trả lời đúng' }}
            </hui-button>
            <hui-button v-if="p.mine && editing !== p.id" size="sm" variant="text" @hui-click="startEdit(p)">Sửa</hui-button>
            <hui-button v-if="p.mine || t.canModerate" size="sm" variant="text" @hui-click="removePost(p)">Xóa</hui-button>
            <hui-button v-if="t.canModerate" size="sm" variant="text" @hui-click="postAction(p, p.hidden ? 'unhide' : 'hide')">{{ p.hidden ? 'Hiện lại' : 'Ẩn' }}</hui-button>
            <hui-button v-if="!p.mine && !t.canModerate && !p.hidden" size="sm" variant="text" :disabled="p.reportedByMe" @hui-click="openReport('post', p.id)">
              {{ p.reportedByMe ? 'Đã báo vi phạm' : 'Báo vi phạm' }}
            </hui-button>
          </div>
        </article>

        <form class="stack reply" @submit.prevent="reply">
          <MarkdownEditor v-model="replyBody" label="Trả lời của bạn" :rows="5" compact :error="replyError" placeholder="Giải thích từng bước; công thức viết trong $…$" />
          <div class="row">
            <hui-button type="submit" :loading="busy" :disabled="!replyBody.trim()">Gửi trả lời</hui-button>
          </div>
        </form>
      </section>
    </template>

    <AskQuestionModal v-if="t" v-model:open="editOpen" :classId="classId" :steps="steps" :thread="t" @saved="(d) => (t = d)" />
    <PromptModal
      v-model:open="reportOpen"
      title="Báo vi phạm"
      label="Lý do (nội dung không phù hợp, xúc phạm, spam…)"
      :saveFn="sendReport"
      @saved="onReported"
    />
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { learnApi, qaApi } from '../../api';
import PromptModal from '../../components/PromptModal.vue';
import AskQuestionModal from '../../components/learning/AskQuestionModal.vue';
import MarkdownEditor from '../../components/teaching/MarkdownEditor.vue';
import { formatDateTime } from '../../utils/form';

const route = useRoute();
const router = useRouter();
const t = ref(null);
const error = ref('');
const steps = ref([]);
const busy = ref(false);
const replyBody = ref('');
const replyError = ref('');
const editing = ref('');
const editBody = ref('');
const editOpen = ref(false);
const reportOpen = ref(false);
const reportTarget = ref(null);

const classId = computed(() => route.params.id);
/** Giáo viên về tab Hỏi đáp của trang quản lý lớp học; học sinh về trang lớp học. */
const backLink = computed(() =>
  t.value?.canModerate
    ? { name: 'teach-class', params: { id: classId.value }, query: { tab: 'hoi-dap' } }
    : { name: 'class', params: { id: classId.value }, hash: '#hoi-dap' },
);

async function load() {
  error.value = '';
  try {
    t.value = await qaApi.get(route.params.threadId);
  } catch (e) {
    error.value = e.status === 404 ? 'Không tìm thấy câu hỏi (có thể đã bị xóa hoặc ẩn).' : e.message;
    t.value = null;
  }
}

async function run(fn, ok) {
  busy.value = true;
  try {
    const res = await fn();
    if (res) t.value = res;
    if (ok) message.success(ok);
    return true;
  } catch (e) {
    message.error(e.message);
    return false;
  } finally {
    busy.value = false;
  }
}

async function reply() {
  replyError.value = '';
  if (!replyBody.value.trim()) return;
  if (await run(() => qaApi.reply(t.value.id, replyBody.value.trim()), 'Đã gửi trả lời.')) replyBody.value = '';
}

const moderate = (action) => run(() => qaApi.moderateThread(t.value.id, action));
const postAction = (p, action) => run(() => qaApi.moderatePost(p.id, action));

function startEdit(p) {
  editing.value = p.id;
  editBody.value = p.body;
}

async function savePost(p) {
  if (await run(() => qaApi.updatePost(p.id, editBody.value.trim()), 'Đã lưu.')) editing.value = '';
}

async function removePost(p) {
  if (!window.confirm('Xóa câu trả lời này?')) return;
  await run(() => qaApi.removePost(p.id), 'Đã xóa trả lời.');
}

async function removeThread() {
  if (!window.confirm('Xóa câu hỏi này?')) return;
  if (await run(() => qaApi.remove(t.value.id).then(() => null), 'Đã xóa câu hỏi.')) router.replace(backLink.value);
}

function openReport(type, id) {
  reportTarget.value = { type, id };
  reportOpen.value = true;
}

const sendReport = (reason) => qaApi.report(reportTarget.value.type, reportTarget.value.id, reason);

function onReported() {
  message.success('Đã gửi báo cáo cho giáo viên.');
  load();
}

watch(() => route.params.threadId, (id) => id && load());

onMounted(async () => {
  await load();
  const learn = await learnApi.learn(classId.value).catch(() => null);
  steps.value = (learn?.steps ?? []).filter((s) => s.kind !== 'milestone').map((s) => ({ id: s.id, title: s.title }));
});
</script>

<style scoped>
.gv {
  margin-left: 4px;
}
.actions {
  gap: 2px;
}
.post {
  display: grid;
  gap: 6px;
  padding: 12px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.post.teacher {
  border-left: 3px solid var(--hui-primary, #0864ad);
}
.post.accepted {
  border-color: var(--hui-success, #10b981);
  background: color-mix(in srgb, var(--hui-success, #10b981) 6%, transparent);
}
.post.hidden {
  opacity: 0.75;
}
.reply {
  margin-top: 8px;
}
</style>

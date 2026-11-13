<template>
  <div class="page">
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="detail">
      <section class="panel stack">
        <div class="row between">
          <h1 style="margin: 0">{{ detail.title }}</h1>
          <div class="row">
            <hui-tag v-if="detail.isOwner" color="primary">Bạn là giáo viên · đang xem như học sinh</hui-tag>
            <hui-tag v-else-if="detail.myStatus" :color="MEMBER_STATUS_COLORS[detail.myStatus]">{{ MEMBER_STATUS_LABELS[detail.myStatus] }}</hui-tag>
          </div>
        </div>
        <p class="muted small">
          {{ subjectName }} · Giáo viên <RouterLink :to="{ name: 'teacher-profile', params: { key: detail.teacherId } }">{{ detail.teacherName }}</RouterLink> · {{ MODE_LABELS[detail.mode] }} · {{ detail.studentCount }} học sinh
        </p>
        <hui-markdown v-if="detail.description" :value="detail.description" />

        <div v-if="!canLearn" class="row">
          <template v-if="detail.myStatus === 'pending'">
            <span class="muted">Yêu cầu ghi danh đang chờ giáo viên duyệt.</span>
          </template>
          <template v-else-if="detail.myStatus === 'removed'">
            <span class="field-error">Bạn đã bị mời ra khỏi lớp học này.</span>
          </template>
          <template v-else-if="detail.enrollment === 'invite'">
            <span class="muted">Lớp học chỉ nhận học sinh có mã mời. <RouterLink to="/kham-pha">Nhập mã mời</RouterLink></span>
          </template>
          <hui-button v-else :loading="enrolling" @hui-click="enroll">
            {{ detail.enrollment === 'open' ? 'Vào học ngay' : 'Gửi yêu cầu ghi danh' }}
          </hui-button>
        </div>
      </section>

      <section v-if="learn && upcoming.length" class="panel stack">
        <div class="row between">
          <h2 style="margin: 0">Buổi học sắp tới</h2>
          <RouterLink to="/lich-hoc">Xem lịch học →</RouterLink>
        </div>
        <ul class="clean-list">
          <li v-for="x in upcoming" :key="x.id" class="row between">
            <span>
              <RouterLink :to="{ name: 'session', params: { id: x.id } }">{{ x.title }}</RouterLink>
              <span class="muted small"> · {{ formatSessionTime(x.startsAt, x.endsAt) }}</span>
            </span>
            <hui-tag size="sm" :color="PHASE_COLORS[sessionPhase(x)]">{{ PHASE_LABELS[sessionPhase(x)] }}</hui-tag>
          </li>
        </ul>
      </section>

      <!-- Đang học: lộ trình có trạng thái -->
      <section v-if="learn" class="panel stack">
        <div class="row between">
          <h2 style="margin: 0">Lộ trình</h2>
          <div class="row">
            <span class="small">Đã xong {{ learn.progress.done }}/{{ learn.progress.total }}</span>
            <div class="progress-bar" style="width: 160px"><span :style="{ width: `${learn.progress.percent}%` }" /></div>
            <b>{{ learn.progress.percent }}%</b>
          </div>
        </div>
        <p v-if="!learn.steps.length" class="muted">Giáo viên chưa xuất bản lộ trình.</p>
        <ol class="path">
          <li v-for="s in learn.steps" :key="s.id" :class="[`kind-${s.kind}`, `st-${s.status}`]">
            <h3 v-if="s.kind === 'milestone'">{{ s.title }}</h3>
            <template v-else>
              <span class="icon" :title="STEP_STATUS_LABELS[s.status]">{{ s.status === 'done' ? '✓' : s.status === 'locked' ? '🔒' : s.kind === 'exam' ? '✎' : s.kind === 'session' ? '◷' : s.kind === 'deck' ? '❏' : s.kind === 'homework' ? '✍' : '▶' }}</span>
              <div class="body">
                <div class="row">
                  <b>{{ s.title }}</b>
                  <hui-tag v-if="s.kind === 'exam'" size="sm" :color="EXAM_KIND_COLORS[s.examKind]">{{ EXAM_KIND_LABELS[s.examKind] }}</hui-tag>
                  <hui-tag v-if="s.kind === 'session'" size="sm" :color="PHASE_COLORS[phaseOf(s)]">{{ PHASE_LABELS[phaseOf(s)] }}</hui-tag>
                  <hui-tag v-if="s.kind === 'deck' && s.dueCards" size="sm" color="warning">{{ s.dueCards }} thẻ cần ôn</hui-tag>
                  <hui-tag v-if="s.kind === 'homework' && s.homeworkStatus" size="sm" :color="HOMEWORK_STATUS_COLORS[s.homeworkStatus]">{{ HOMEWORK_STATUS_LABELS[s.homeworkStatus] }}</hui-tag>
                  <hui-tag v-if="s.restricted" size="sm" color="secondary" title="Bước chỉ dành cho một số nhóm / học sinh">Riêng</hui-tag>
                </div>
                <span class="muted small">
                  <template v-if="s.kind === 'lesson'">Bài học{{ s.durationMinutes ? ` · ${s.durationMinutes} phút` : '' }}</template>
                  <template v-else-if="s.kind === 'session'">Buổi học · {{ formatSessionTime(s.startsAt, s.endsAt) }}<template v-if="s.status === 'done'"> · đã điểm danh</template></template>
                  <template v-else-if="s.kind === 'deck'">Thẻ ghi nhớ · {{ s.itemCount }} thẻ<template v-if="s.status === 'done'"> · đã ôn hết một lượt</template></template>
                  <template v-else-if="s.kind === 'homework'">
                    Bài tập về nhà<template v-if="s.dueAt"> · hạn nộp <span :class="{ 'field-error': !s.homeworkStatus && new Date(s.dueAt) < new Date() }">{{ formatDateTime(s.dueAt) }}</span></template>
                    <template v-if="s.homeworkScore != null"> · {{ formatScore(s.homeworkScore) }}/{{ formatScore(s.maxScore) }} điểm</template>
                  </template>
                  <template v-else>
                    {{ s.itemCount }} câu{{ s.timeLimitMinutes ? ` · ${s.timeLimitMinutes} phút` : '' }}
                    <template v-if="s.passScore != null"> · cần đạt {{ formatScore(s.passScore) }} điểm</template>
                    <template v-if="s.submittedCount"> · đã làm {{ s.submittedCount }} lần, cao nhất {{ formatScore(s.bestScore) }}/{{ formatScore(s.maxScore) }}</template>
                  </template>
                  <template v-if="s.status === 'locked'"> · hoàn thành bước trước để mở</template>
                </span>
                <ul v-if="attemptsOf[s.id]" class="clean-list small attempts">
                  <li v-for="a in attemptsOf[s.id]" :key="a.id">
                    <RouterLink :to="{ name: 'attempt', params: { id: a.id } }">{{ formatDateTime(a.startedAt) }}</RouterLink>
                    — {{ a.status === 'submitted' ? `${formatScore(a.score)}/${formatScore(a.maxScore)} điểm` : 'đang làm' }}
                  </li>
                </ul>
              </div>
              <div class="actions">
                <template v-if="s.status !== 'locked'">
                  <RouterLink v-if="s.kind === 'lesson'" :to="{ name: 'lesson-view', params: { id: detail.id, stepId: s.id } }">
                    {{ s.status === 'done' ? 'Xem lại' : 'Học' }}
                  </RouterLink>
                  <RouterLink v-else-if="s.kind === 'homework'" :to="{ name: 'homework', params: { id: detail.id, stepId: s.id } }">
                    {{ !s.homeworkStatus ? 'Làm bài' : s.homeworkStatus === 'returned' ? 'Làm lại' : 'Xem bài' }}
                  </RouterLink>
                  <RouterLink v-else-if="s.kind === 'deck'" :to="{ name: 'deck-study', params: { id: detail.id, stepId: s.id } }">
                    {{ s.dueCards ? 'Ôn thẻ' : 'Luyện thêm' }}
                  </RouterLink>
                  <RouterLink v-else-if="s.kind === 'session'" :to="{ name: 'session', params: { id: s.sessionId } }">
                    {{ ['live', 'soon'].includes(phaseOf(s)) ? 'Vào học' : 'Chi tiết' }}
                  </RouterLink>
                  <template v-else>
                    <hui-button size="sm" :loading="starting === s.id" @hui-click="startExam(s)">
                      {{ s.inProgressAttemptId ? 'Làm tiếp' : s.submittedCount ? 'Làm lại' : 'Làm bài' }}
                    </hui-button>
                    <hui-button v-if="s.submittedCount" size="sm" variant="text" @hui-click="toggleAttempts(s)">
                      {{ attemptsOf[s.id] ? 'Ẩn' : 'Các lần làm' }}
                    </hui-button>
                  </template>
                </template>
              </div>
            </template>
          </li>
        </ol>
        <div v-if="detail.myStatus === 'active'" class="row">
          <hui-button size="sm" variant="text" @hui-click="leave">Rời lớp học</hui-button>
        </div>
      </section>

      <section v-if="learn" id="hoi-dap" class="panel">
        <QaPanel :classId="detail.id" :teacher="detail.isOwner" />
      </section>

      <!-- Chưa vào học: mục lục -->
      <section v-else-if="detail.outline.length" class="panel stack">
        <h2 style="margin: 0">Nội dung lớp học</h2>
        <ol class="outline">
          <li v-for="(o, i) in detail.outline" :key="i" :class="`kind-${o.kind}`">
            <b v-if="o.kind === 'milestone'">{{ o.title }}</b>
            <span v-else>{{ STEP_KIND_LABELS[o.kind] }}: {{ o.title }}<span v-if="o.durationMinutes" class="muted small"> · {{ o.durationMinutes }} phút</span></span>
          </li>
        </ol>
      </section>
    </template>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, nextTick, onMounted, reactive, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';

import { learnApi, sessionApi } from '../../api';
import QaPanel from '../../components/learning/QaPanel.vue';
import { formatDateTime } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { PHASE_COLORS, PHASE_LABELS, formatSessionTime, sessionPhase } from '../../utils/schedule';
import {
  EXAM_KIND_COLORS,
  EXAM_KIND_LABELS,
  MEMBER_STATUS_COLORS,
  MEMBER_STATUS_LABELS,
  MODE_LABELS,
  STEP_KIND_LABELS,
  HOMEWORK_STATUS_COLORS,
  HOMEWORK_STATUS_LABELS,
  STEP_STATUS_LABELS,
  formatScore,
} from '../../utils/teaching';

const route = useRoute();
const router = useRouter();
const { data: master, load: loadMaster } = useMasterData();
const detail = ref(null);
const learn = ref(null);
const error = ref('');
const enrolling = ref(false);
const starting = ref('');
const attemptsOf = reactive({});
/** Buổi học sắp tới của lớp học (30 ngày) */
const upcoming = ref([]);
const phaseOf = (s) => sessionPhase({ status: s.sessionStatus, startsAt: s.startsAt, endsAt: s.endsAt });

const canLearn = computed(() => detail.value?.isOwner || detail.value?.myStatus === 'active');
const subjectName = computed(() => master.value?.subjects.find((s) => s.id === detail.value?.subjectId)?.name ?? '');

async function load() {
  error.value = '';
  try {
    detail.value = await learnApi.detail(route.params.id);
    learn.value = canLearn.value ? await learnApi.learn(route.params.id) : null;
    if (learn.value && !detail.value.isOwner) {
      const now = Date.now();
      upcoming.value = (
        await sessionApi
          .mine({ classId: route.params.id, from: new Date(now - 3 * 3_600_000).toISOString(), to: new Date(now + 30 * 86_400_000).toISOString() })
          .catch(() => [])
      )
        .filter((x) => x.status !== 'cancelled' && new Date(x.endsAt) > now)
        .slice(0, 5);
    }
    if (route.hash === '#hoi-dap') nextTick(() => document.getElementById('hoi-dap')?.scrollIntoView());
  } catch (e) {
    error.value = e.message;
  }
}

async function enroll() {
  enrolling.value = true;
  try {
    const res = await learnApi.enroll(detail.value.id);
    message.success(res.status === 'active' ? 'Đã vào lớp học.' : 'Đã gửi yêu cầu, chờ giáo viên duyệt.');
    await load();
  } catch (e) {
    message.error(e.message);
  } finally {
    enrolling.value = false;
  }
}

async function leave() {
  if (!window.confirm('Rời lớp học? Tiến độ vẫn được giữ nếu bạn vào lại.')) return;
  await learnApi.leave(detail.value.id);
  router.push({ name: 'my-classes' });
}

async function startExam(s) {
  starting.value = s.id;
  try {
    const res = await learnApi.startExam(detail.value.id, s.id);
    router.push({ name: 'attempt', params: { id: res.attemptId } });
  } catch (e) {
    message.error(e.message);
  } finally {
    starting.value = '';
  }
}

async function toggleAttempts(s) {
  if (attemptsOf[s.id]) {
    delete attemptsOf[s.id];
    return;
  }
  attemptsOf[s.id] = await learnApi.stepAttempts(detail.value.id, s.id).catch(() => []);
}

onMounted(() => {
  loadMaster();
  load();
});
</script>

<style scoped>
.path {
  list-style: none;
  margin: 0;
  padding: 0;
  display: grid;
  gap: 8px;
}
.path li {
  display: grid;
  grid-template-columns: 32px 1fr auto;
  gap: 10px;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
.path li.kind-milestone {
  display: block;
  border: 0;
  padding: 10px 0 0;
}
.path li.kind-milestone h3 {
  margin: 0;
  font-size: 1rem;
  color: var(--hui-text-secondary, #6b7280);
}
.path li.st-done {
  background: color-mix(in srgb, var(--hui-success, #52c41a) 8%, transparent);
}
.path li.st-locked {
  opacity: 0.6;
}
.icon {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  background: var(--hui-bg-secondary, #f5f6f8);
}
.st-done .icon {
  background: var(--hui-success, #52c41a);
  color: #fff;
}
.body {
  display: grid;
  gap: 2px;
}
.actions {
  display: flex;
  gap: 6px;
  align-items: center;
}
.attempts {
  margin-top: 4px;
  gap: 2px;
}
.outline {
  margin: 0;
  padding-left: 1.4em;
  display: grid;
  gap: 4px;
}
.outline li.kind-milestone {
  list-style: none;
  margin-left: -1.4em;
  margin-top: 6px;
}
@media (max-width: 640px) {
  .path li {
    grid-template-columns: 28px 1fr;
  }
  .actions {
    grid-column: 2;
  }
}
</style>

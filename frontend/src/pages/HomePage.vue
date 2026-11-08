<template>
  <div class="page">
    <div class="page-header">
      <h1>Chào {{ auth.user?.fullName }}</h1>
    </div>

    <div class="home-grid">
      <!-- Lịch 7 ngày tới -->
      <section class="panel stack">
        <div class="row between">
          <h2 style="margin: 0">Lịch 7 ngày tới</h2>
          <RouterLink :to="isTeacher ? '/day-hoc/lich' : '/lich-hoc'">Xem lịch →</RouterLink>
        </div>
        <p v-if="sessionsLoaded && !upcoming.length" class="muted" style="margin: 0">
          Không có buổi học nào trong 7 ngày tới.
          <RouterLink v-if="isTeacher" to="/day-hoc/lich">Tạo buổi học</RouterLink>
        </p>
        <ul class="clean-list">
          <li v-for="x in upcoming" :key="x.id" class="item">
            <div class="when">
              <b>{{ dayLabel(x.startsAt) }}</b>
              <span class="muted small">{{ timeRange(x) }}</span>
            </div>
            <div class="what">
              <RouterLink :to="{ name: 'session', params: { id: x.id } }"><b>{{ x.title }}</b></RouterLink>
              <span class="muted small">
                {{ x.isOwner ? 'Bạn dạy' : `Giáo viên ${x.teacherName}` }}<template v-if="x.classTitle"> · {{ x.classTitle }}</template>
              </span>
            </div>
            <div class="row tags">
              <hui-tag size="sm" :color="PHASE_COLORS[sessionPhase(x, now)]">
                {{ PHASE_LABELS[sessionPhase(x, now)] }}<template v-if="timeUntil(x.startsAt, now)"> · {{ timeUntil(x.startsAt, now) }}</template>
              </hui-tag>
              <RouterLink v-if="['live', 'soon'].includes(sessionPhase(x, now))" class="join" :to="{ name: 'session', params: { id: x.id } }">Vào học</RouterLink>
            </div>
          </li>
        </ul>
      </section>

      <!-- Học tiếp -->
      <section class="panel stack">
        <div class="row between">
          <h2 style="margin: 0">Học tiếp</h2>
          <RouterLink to="/lop-hoc-cua-toi">Lớp học của tôi →</RouterLink>
        </div>
        <p v-if="classesLoaded && !learning.length" class="muted" style="margin: 0">
          Bạn chưa vào lớp học nào. <RouterLink to="/kham-pha">Tìm lớp học</RouterLink>
        </p>
        <ul class="clean-list">
          <li v-for="m in learning" :key="m.class.id" class="stack" style="gap: 4px">
            <div class="row between">
              <RouterLink :to="{ name: 'class', params: { id: m.class.id } }"><b>{{ m.class.title }}</b></RouterLink>
              <span class="small muted">{{ m.progress.done }}/{{ m.progress.total }}</span>
            </div>
            <div class="progress-bar"><span :style="{ width: `${m.progress.percent}%` }" /></div>
          </li>
        </ul>
      </section>
    </div>

    <section class="panel">
      <p style="margin-top: 0">
        Bạn đang đăng nhập với vai trò <strong>{{ ROLE_LABELS[auth.user?.role] }}</strong>.
      </p>
      <ul class="shortcuts">
        <li v-for="item in shortcuts" :key="item.to">
          <RouterLink :to="item.to">{{ item.label }}</RouterLink>
        </li>
      </ul>
    </section>

    <section v-if="auth.can(P.TEACHER_SUBMIT) && latestApplication" class="panel">
      <h2>Hồ sơ giáo viên của bạn</h2>
      <p>
        Trạng thái:
        <hui-tag :color="APP_STATUS_COLORS[latestApplication.status]">{{ APP_STATUS_LABELS[latestApplication.status] }}</hui-tag>
      </p>
      <p v-if="latestApplication.rejectReason" class="muted">Lý do: {{ latestApplication.rejectReason }}</p>
    </section>
  </div>
</template>

<script setup>
import { computed, onMounted, onUnmounted, ref } from 'vue';

import { learnApi, sessionApi, teacherApi, teachingApi } from '../api';
import { useAuthStore } from '../stores/auth';
import { APP_STATUS_COLORS, APP_STATUS_LABELS } from '../utils/labels';
import { P, ROLE_LABELS, visibleMenu } from '../utils/permissions';
import { PHASE_COLORS, PHASE_LABELS, formatDay, mergeSessions, sessionPhase, timeUntil } from '../utils/schedule';

const DAY = 86_400_000;
const auth = useAuthStore();
const isTeacher = computed(() => auth.can(P.TEACHING_MANAGE));
const applications = ref([]);
const latestApplication = computed(() => applications.value[0] ?? null);
const shortcuts = computed(() =>
  visibleMenu(auth.permissions)
    .flatMap((g) => g.items)
    .filter((i) => i.to !== '/'),
);

const now = ref(Date.now());
const sessions = ref([]);
const sessionsLoaded = ref(false);
const upcoming = computed(() =>
  sessions.value.filter((x) => x.status !== 'cancelled' && new Date(x.endsAt) > now.value).slice(0, 8),
);
const myClasses = ref([]);
const classesLoaded = ref(false);
/** Lớp đang học, chưa xong — nhiều tiến độ nhất trước */
const learning = computed(() =>
  myClasses.value
    .filter((m) => m.class.myStatus === 'active' && m.progress && m.progress.total > m.progress.done)
    .sort((a, b) => b.progress.percent - a.progress.percent)
    .slice(0, 5),
);
let timer = null;

const pad = (n) => String(n).padStart(2, '0');
const hm = (v) => {
  const d = new Date(v);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
};
const timeRange = (x) => `${hm(x.startsAt)} – ${hm(x.endsAt)}`;
function dayLabel(v) {
  const d = new Date(v);
  const today = new Date(now.value);
  const diff = Math.round((new Date(d.toDateString()) - new Date(today.toDateString())) / DAY);
  if (diff === 0) return 'Hôm nay';
  if (diff === 1) return 'Ngày mai';
  return formatDay(v).replace(/\/\d{4}$/, '');
}

async function loadSessions() {
  const t = Date.now();
  const range = { from: new Date(t - 6 * 3_600_000).toISOString(), to: new Date(t + 7 * DAY).toISOString() };
  const [mine, teaching] = await Promise.all([
    sessionApi.mine(range).catch(() => []),
    isTeacher.value ? teachingApi.sessions(range).catch(() => []) : [],
  ]);
  sessions.value = mergeSessions(mine, teaching);
  sessionsLoaded.value = true;
}

onMounted(async () => {
  timer = setInterval(() => (now.value = Date.now()), 30_000);
  loadSessions();
  learnApi
    .myClasses()
    .then((list) => (myClasses.value = list))
    .catch(() => {})
    .finally(() => (classesLoaded.value = true));
  if (auth.can(P.TEACHER_SUBMIT)) {
    try {
      applications.value = await teacherApi.mine();
    } catch {
      applications.value = [];
    }
  }
});
onUnmounted(() => clearInterval(timer));
</script>

<style scoped>
.home-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(340px, 100%), 1fr));
  gap: 16px;
  align-items: start;
}
.item {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  grid-template-areas:
    'when tags'
    'what what';
  gap: 4px 12px;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.item:last-child {
  border-bottom: 0;
}
.when,
.what {
  display: grid;
  gap: 2px;
  min-width: 0;
}
.when {
  grid-area: when;
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  align-items: baseline;
}
.what {
  grid-area: what;
}
.tags {
  grid-area: tags;
}
.join {
  font-weight: 600;
  color: var(--hui-success, #10b981);
}
.shortcuts {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 20px;
  padding: 0;
  margin-bottom: 0;
  list-style: none;
}
</style>

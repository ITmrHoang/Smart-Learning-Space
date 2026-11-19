<template>
  <div class="page">
    <hui-alert v-if="error" severity="error" :description="error" />
    <template v-if="s">
      <section class="panel stack">
        <div class="row between">
          <div class="stack" style="gap: 4px">
            <div class="row">
              <h1 style="margin: 0">{{ s.title }}</h1>
              <hui-tag :color="PHASE_COLORS[phase]">{{ PHASE_LABELS[phase] }}</hui-tag>
              <hui-tag v-if="s.scheduleId" size="sm" color="secondary">Lịch lặp hằng tuần</hui-tag>
            </div>
            <p class="muted" style="margin: 0">
              {{ formatSessionTime(s.startsAt, s.endsAt) }} · {{ formatDuration(s.durationMinutes) }}
              <template v-if="untilText"> · <b>{{ untilText }}</b></template>
            </p>
            <p class="muted small" style="margin: 0">
              Giáo viên {{ s.teacherName }}
              <template v-if="s.classTitle">
                · Lớp học
                <RouterLink :to="s.isOwner ? { name: 'teach-class', params: { id: s.classId } } : { name: 'class', params: { id: s.classId } }">{{ s.classTitle }}</RouterLink>
              </template>
              · {{ ACCESS_LABELS[s.access] }}
            </p>
          </div>
          <div v-if="s.isOwner && s.status !== 'cancelled'" class="row">
            <hui-button variant="outline" @hui-click="formOpen = true">Sửa</hui-button>
            <hui-button variant="danger" @hui-click="cancelOpen = true">Hủy buổi</hui-button>
          </div>
        </div>

        <!-- Vào học: phòng của hệ thống -->
        <div v-if="s.room === 'live' && s.status !== 'cancelled' && phase !== 'ended'" class="join">
          <template v-if="s.canJoin || (s.isOwner && teacherCanOpen)">
            <RouterLink class="join-btn" :to="{ name: 'live-room', params: { id: s.id } }">{{ s.isOwner ? 'Mở phòng học' : 'Vào học' }}</RouterLink>
            <span class="muted small">Phòng học trực tuyến của hệ thống: video, bảng, giơ tay, bình chọn.</span>
          </template>
          <template v-else>
            <span class="join-btn disabled" aria-disabled="true">Vào học</span>
            <span class="muted small">
              {{ s.isOwner ? 'Mở phòng được từ 60 phút trước giờ học' : `Mở lúc ${formatTime(s.joinOpensAt)} (${JOIN_EARLY_MINUTES} phút trước giờ học)` }} ·
              {{ timeUntil(s.startsAt, now) }}
            </span>
          </template>
        </div>
        <!-- Vào học: link họp ngoài -->
        <div v-else-if="s.status !== 'cancelled' && phase !== 'ended'" class="join">
          <template v-if="s.canJoin">
            <a v-if="s.meetingUrl" class="join-btn" :href="s.meetingUrl" target="_blank" rel="noopener" @click="join">Vào học</a>
            <hui-button v-else :loading="joining" @hui-click="join">Vào học (điểm danh)</hui-button>
            <span class="muted small">
              {{ s.meetingUrl ? 'Mở phòng học ở tab mới và ghi điểm danh.' : 'Giáo viên chưa gắn link phòng học — bấm để điểm danh.' }}
            </span>
          </template>
          <template v-else>
            <span class="join-btn disabled" aria-disabled="true">Vào học</span>
            <span class="muted small">Mở lúc {{ formatTime(s.joinOpensAt) }} ({{ JOIN_EARLY_MINUTES }} phút trước giờ học) · {{ timeUntil(s.startsAt, now) }}</span>
          </template>
        </div>
        <p v-if="s.attended" class="small"><hui-tag size="sm" color="success">Đã điểm danh</hui-tag></p>

        <!-- Trả lời tham gia (học sinh được mời) -->
        <div v-if="s.invited && !s.isOwner && s.status !== 'cancelled' && phase !== 'ended'" class="row">
          <span class="small">Bạn sẽ tham gia?</span>
          <hui-button size="sm" :variant="s.myRsvp === 'yes' ? 'primary' : 'outline'" :loading="rsvping === 'yes'" @hui-click="rsvp('yes')">Tham gia</hui-button>
          <hui-button size="sm" :variant="s.myRsvp === 'no' ? 'secondary' : 'outline'" :loading="rsvping === 'no'" @hui-click="rsvp('no')">Không tham gia</hui-button>
          <span v-if="s.myRsvp" class="muted small">Đã trả lời: {{ RSVP_LABELS[s.myRsvp] }}</span>
        </div>

        <hui-markdown v-if="s.description" :value="s.description" />
        <div v-if="s.lessons.length">
          <h2 class="h3">Bài sẽ dạy</h2>
          <ul class="clean-list small">
            <li v-for="l in s.lessons" :key="l.id">
              <RouterLink :to="{ name: 'session-lesson', params: { id: s.id, lessonId: l.id } }">{{ l.name }}</RouterLink>
              <RouterLink v-if="s.isOwner" :to="{ name: 'teach-lesson', params: { id: l.id } }" class="muted small"> · sửa bài</RouterLink>
            </li>
          </ul>
        </div>
      </section>

      <LiveRecords v-if="s.room === 'live'" :sessionId="s.id" />

      <!-- Giáo viên: người được mời, trả lời, điểm danh -->
      <section v-if="s.isOwner" class="panel stack">
        <div class="row between">
          <h2 style="margin: 0">Học sinh ({{ s.attendees.length }})</h2>
          <span class="muted small">
            Tham gia {{ count('yes') }} · Không tham gia {{ count('no') }} · Chưa trả lời {{ s.attendees.length - count('yes') - count('no') }} ·
            Có mặt {{ s.attendees.filter((a) => a.attended).length }}
          </span>
        </div>
        <p class="muted small" style="margin: 0">
          Mời: <template v-for="(name, i) in inviteeNames" :key="i">{{ i ? ', ' : '' }}{{ name }}</template>
          <template v-if="!inviteeNames.length">chưa mời ai</template>
        </p>
        <p v-if="!s.attendees.length" class="muted">Chưa có học sinh nào được mời.</p>
        <table v-else class="table">
          <thead>
            <tr><th>Học sinh</th><th>Trả lời</th><th>Vào học lúc</th><th>Có mặt</th></tr>
          </thead>
          <tbody>
            <tr v-for="a in s.attendees" :key="a.userId">
              <td>{{ a.fullName }} <span class="muted small">@{{ a.username }}</span></td>
              <td>
                <hui-tag v-if="a.rsvp" size="sm" :color="RSVP_COLORS[a.rsvp]">{{ RSVP_LABELS[a.rsvp] }}</hui-tag>
                <span v-else class="muted small">—</span>
              </td>
              <td class="small">{{ a.joinedAt ? formatDateTime(a.joinedAt) : '—' }}</td>
              <td>
                <hui-checkbox :checked="a.attended" :disabled="s.status === 'cancelled'" :label="a.attended ? 'Có mặt' : 'Vắng'" @change="setAttended(a, $event.detail.checked)" />
              </td>
            </tr>
          </tbody>
        </table>
        <p class="muted small" style="margin: 0">Học sinh bấm "Vào học" được tự điểm danh. Bước "buổi học" trong lộ trình xong khi học sinh có mặt.</p>
      </section>
    </template>

    <SessionFormModal v-if="s?.isOwner" v-model:open="formOpen" :session="s" @saved="load" />
    <hui-modal :open="cancelOpen" title="Hủy buổi học" ok-text="Hủy buổi" ok-type="danger" cancel-text="Không" :confirmLoading="cancelling" @hui-ok="cancel" @hui-close="cancelOpen = false">
      <div class="stack">
        <p style="margin: 0">Học sinh được mời sẽ nhận thông báo và email hủy lịch.</p>
        <hui-radio-group v-if="s?.scheduleId" :options="scopeOptions" :value="cancelScope" @change="cancelScope = $event.detail.value" />
      </div>
    </hui-modal>
  </div>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useRoute } from 'vue-router';

import { sessionApi, teachingApi } from '../../api';
import LiveRecords from '../../components/live/LiveRecords.vue';
import SessionFormModal from '../../components/teaching/SessionFormModal.vue';
import { formatDateTime } from '../../utils/form';
import {
  ACCESS_LABELS,
  JOIN_EARLY_MINUTES,
  PHASE_COLORS,
  PHASE_LABELS,
  RSVP_COLORS,
  RSVP_LABELS,
  formatDuration,
  formatSessionTime,
  sessionPhase,
  timeUntil,
} from '../../utils/schedule';

const route = useRoute();
const s = ref(null);
const error = ref('');
const now = ref(Date.now());
const joining = ref(false);
const rsvping = ref('');
const formOpen = ref(false);
const cancelOpen = ref(false);
const cancelling = ref(false);
const cancelScope = ref('this');
const scopeOptions = [
  { value: 'this', label: 'Chỉ buổi này' },
  { value: 'following', label: 'Buổi này và các buổi sau' },
];
let timer = null;

const phase = computed(() => (s.value ? sessionPhase(s.value, now.value) : 'upcoming'));
const untilText = computed(() => (phase.value === 'upcoming' || phase.value === 'soon' ? timeUntil(s.value.startsAt, now.value) : ''));
/** Giáo viên mở phòng học của hệ thống được từ 60 phút trước giờ học (khớp backend). */
const teacherCanOpen = computed(() => !!s.value && now.value >= new Date(s.value.startsAt).getTime() - 60 * 60_000);
const inviteeNames = computed(() => {
  const v = s.value?.invitees;
  if (!v) return [];
  return [...v.classes.map((x) => `lớp học ${x.name}`), ...v.groups.map((x) => `nhóm ${x.name}`), ...v.users.map((x) => x.name)];
});
const count = (r) => s.value.attendees.filter((a) => a.rsvp === r).length;
const formatTime = (v) => new Date(v).toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' });

async function load() {
  error.value = '';
  try {
    s.value = await sessionApi.detail(route.params.id);
  } catch (e) {
    error.value = e.status === 404 ? 'Không tìm thấy buổi học, hoặc bạn không được mời.' : e.message;
    s.value = null;
  }
}

async function join() {
  joining.value = true;
  try {
    await sessionApi.join(s.value.id);
    if (!s.value.isOwner) {
      s.value.attended = true;
      if (!s.value.meetingUrl) message.success('Đã điểm danh.');
    }
  } catch (e) {
    message.error(e.message);
  } finally {
    joining.value = false;
  }
}

async function rsvp(response) {
  rsvping.value = response;
  try {
    await sessionApi.rsvp(s.value.id, response);
    s.value.myRsvp = response;
  } catch (e) {
    message.error(e.message);
  } finally {
    rsvping.value = '';
  }
}

async function setAttended(a, attended) {
  const old = a.attended;
  a.attended = attended;
  try {
    await teachingApi.setAttendance(s.value.id, a.userId, attended);
  } catch (e) {
    a.attended = old;
    message.error(e.message);
  }
}

async function cancel(e) {
  e.preventDefault();
  cancelling.value = true;
  try {
    const res = await teachingApi.cancelSession(s.value.id, cancelScope.value);
    message.success(res.updated > 1 ? `Đã hủy ${res.updated} buổi học.` : 'Đã hủy buổi học.');
    cancelOpen.value = false;
    await load();
  } catch (err) {
    message.error(err.message);
  } finally {
    cancelling.value = false;
  }
}

watch(() => route.params.id, (id) => id && load());

onMounted(() => {
  load();
  // Nút "Vào học" tự mở khi tới giờ
  timer = setInterval(() => {
    const before = s.value && s.value.canJoin;
    now.value = Date.now();
    if (s.value && !before && new Date(s.value.joinOpensAt) <= now.value && now.value <= new Date(s.value.endsAt)) load();
  }, 30_000);
});
onUnmounted(() => clearInterval(timer));
</script>

<style scoped>
.join {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
}
.join-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  height: var(--hui-size-lg, 38px);
  padding: 0 22px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-success, #10b981);
  color: #fff;
  font-weight: 600;
  text-decoration: none;
}
.join-btn:hover {
  text-decoration: none;
  filter: brightness(1.05);
}
.join-btn.disabled {
  background: var(--hui-fill, #e5e7eb);
  color: var(--hui-text-secondary, #6b7280);
  cursor: not-allowed;
}
.h3 {
  font-size: 1.05rem;
}
.table {
  width: 100%;
  border-collapse: collapse;
}
.table th,
.table td {
  text-align: left;
  padding: 8px;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.table th {
  font-size: 0.8125rem;
  color: var(--hui-text-secondary, #6b7280);
  font-weight: 500;
}
</style>

<template>
  <div class="page">
    <div class="page-header">
      <h1>Lịch học</h1>
    </div>
    <p class="muted small">Các buổi học bạn được mời. Bấm vào buổi để xem chi tiết, trả lời tham gia và "Vào học" (mở trước giờ học {{ JOIN_EARLY_MINUTES }} phút).</p>

    <section v-if="next.length" class="panel stack">
      <h2 style="margin: 0">Sắp diễn ra</h2>
      <ul class="clean-list">
        <li v-for="x in next" :key="x.id" class="row between item">
          <span>
            <RouterLink :to="{ name: 'session', params: { id: x.id } }"><b>{{ x.title }}</b></RouterLink>
            <span class="muted small"> · {{ formatSessionTime(x.startsAt, x.endsAt) }}<template v-if="x.classTitle"> · {{ x.classTitle }}</template></span>
          </span>
          <span class="row">
            <hui-tag v-if="x.myRsvp" size="sm" :color="RSVP_COLORS[x.myRsvp]">{{ RSVP_LABELS[x.myRsvp] }}</hui-tag>
            <hui-tag size="sm" :color="PHASE_COLORS[sessionPhase(x)]">{{ PHASE_LABELS[sessionPhase(x)] }}<template v-if="timeUntil(x.startsAt)"> · {{ timeUntil(x.startsAt) }}</template></hui-tag>
          </span>
        </li>
      </ul>
    </section>

    <ScheduleCalendar :fetcher="sessionApi.mine" @open="(id) => router.push({ name: 'session', params: { id } })" />
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

import { sessionApi } from '../../api';
import ScheduleCalendar from '../../components/ScheduleCalendar.vue';
import {
  JOIN_EARLY_MINUTES,
  PHASE_COLORS,
  PHASE_LABELS,
  RSVP_COLORS,
  RSVP_LABELS,
  formatSessionTime,
  sessionPhase,
  timeUntil,
} from '../../utils/schedule';

const router = useRouter();
/** 7 ngày tới (không tính buổi đã hủy / đã kết thúc) */
const next = ref([]);

onMounted(async () => {
  const now = Date.now();
  const list = await sessionApi
    .mine({ from: new Date(now - 6 * 3_600_000).toISOString(), to: new Date(now + 7 * 86_400_000).toISOString() })
    .catch(() => []);
  next.value = list.filter((x) => x.status !== 'cancelled' && new Date(x.endsAt) > now).slice(0, 8);
});
</script>

<style scoped>
.item {
  padding: 8px 0;
  border-bottom: 1px solid var(--hui-border-secondary, #eceef1);
}
.item:last-child {
  border-bottom: 0;
}
</style>

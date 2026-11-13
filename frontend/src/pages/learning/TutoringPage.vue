<template>
  <div class="page">
    <div class="page-header"><h1>Học kèm 1-1</h1></div>
    <p class="muted small">Chọn giờ rảnh của giáo viên để học riêng. Giáo viên nhận thì buổi học hiện trong "Lịch học" kèm link vào học.</p>
    <hui-alert v-if="error" severity="error" :description="error" />

    <section class="panel stack">
      <div class="row between">
        <h2 style="margin: 0">Lịch học kèm của tôi</h2>
        <label class="small"><input v-model="showAll" type="checkbox" @change="loadMine" /> Xem cả lịch cũ / đã hủy</label>
      </div>
      <p v-if="!mine.length" class="muted small" style="margin: 0">Chưa có lịch học kèm nào.</p>
      <article v-for="b in mine" :key="b.id" class="booking">
        <div class="stack" style="gap: 2px">
          <span class="row">
            <b>{{ formatSessionTime(b.startsAt, b.endsAt) }}</b>
            <hui-tag size="sm" :color="BOOKING_STATUS_COLORS[b.status]">{{ BOOKING_STATUS_LABELS[b.status] }}</hui-tag>
          </span>
          <span class="small">{{ b.offerTitle }} · {{ b.teacherName }}</span>
          <span v-if="b.reply" class="small muted">Lý do: {{ b.reply }}</span>
        </div>
        <span class="row">
          <RouterLink v-if="b.sessionId && b.status === 'approved'" :to="{ name: 'session', params: { id: b.sessionId } }" class="small">Vào buổi học →</RouterLink>
          <hui-button v-if="cancellable(b)" size="sm" variant="text" @hui-click="ask(b)">Hủy</hui-button>
        </span>
      </article>
    </section>

    <section class="stack">
      <h2 style="margin: 0">Giáo viên đang mở lịch</h2>
      <p v-if="loaded && !offers.length" class="panel empty muted">Chưa có giáo viên nào mở lịch học kèm cho bạn.</p>
      <div class="card-list">
        <article v-for="o in offers" :key="o.id" class="card">
          <h3 style="margin: 0">{{ o.title }}</h3>
          <RouterLink :to="{ name: 'teacher-profile', params: { key: o.teacherUsername } }" class="small">{{ o.teacherName }}</RouterLink>
          <div class="row">
            <hui-tag size="sm">{{ formatDuration(o.durationMinutes) }} / buổi</hui-tag>
          </div>
          <p class="small" style="margin: 0">{{ formatWindows(o.windows) }}</p>
          <p v-if="o.description" class="muted small" style="margin: 0">{{ o.description }}</p>
          <hui-button size="sm" @hui-click="book(o)">Chọn giờ</hui-button>
        </article>
      </div>
    </section>

    <BookingModal v-model:open="bookingOpen" :offer="selected" @booked="loadMine" />
    <PromptModal v-model:open="cancelOpen" title="Hủy lịch học kèm" label="Lý do (giáo viên sẽ thấy)" :saveFn="sendCancel" @saved="loadMine" />
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';

import { tutoringApi } from '../../api';
import PromptModal from '../../components/PromptModal.vue';
import BookingModal from '../../components/learning/BookingModal.vue';
import { formatDuration, formatSessionTime, formatWindows } from '../../utils/schedule';
import { BOOKING_STATUS_COLORS, BOOKING_STATUS_LABELS } from '../../utils/teaching';

const offers = ref([]);
const mine = ref([]);
const loaded = ref(false);
const error = ref('');
const showAll = ref(false);
const bookingOpen = ref(false);
const selected = ref(null);
const cancelOpen = ref(false);
const target = ref(null);

const cancellable = (b) => ['pending', 'approved'].includes(b.status) && new Date(b.startsAt) > new Date();

async function loadMine() {
  mine.value = await tutoringApi.mine(showAll.value ? 'all' : 'upcoming').catch(() => []);
}

function book(o) {
  selected.value = o;
  bookingOpen.value = true;
}

function ask(b) {
  target.value = b;
  cancelOpen.value = true;
}

const sendCancel = (reason) => tutoringApi.cancel(target.value.id, reason);

onMounted(async () => {
  try {
    offers.value = await tutoringApi.offers();
  } catch (e) {
    error.value = e.message;
  } finally {
    loaded.value = true;
  }
  loadMine();
});
</script>

<style scoped>
.booking {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  justify-content: space-between;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
}
</style>

<template>
  <div class="schedule-calendar">
    <hui-alert v-if="error" severity="error" :description="error" />
    <hui-timetable
      ref="el"
      :events="events"
      :view="view"
      :selectable="selectable"
      :startHour="6"
      :endHour="22"
      :slotMinutes="30"
      label="Lịch buổi học"
      @hui-range-change="onRange"
      @hui-slot-select="emit('select', $event.detail)"
      @hui-event-click="emit('open', $event.detail.event.id)"
    />
    <p class="legend muted small">
      <span><i class="dot primary" /> Buổi bạn dạy</span>
      <span><i class="dot success" /> Buổi được mời</span>
      <span><i class="dot secondary" /> Đã báo không tham gia</span>
      <span><i class="dot cancelled" /> Đã hủy</span>
      <span v-if="selectable">· Kéo trên ô trống để tạo buổi học</span>
    </p>
  </div>
</template>

<script setup>
import { computed, nextTick, onMounted, ref } from 'vue';

import { toTimetableEvents } from '../utils/schedule';

const props = defineProps({
  /** ({ from, to }: ISO) => Promise<SessionSummary[]> */
  fetcher: { type: Function, required: true },
  selectable: { type: Boolean, default: false },
});
const emit = defineEmits(['select', 'open', 'loaded']);

const el = ref(null);
const sessions = ref(null);
const error = ref('');
/** Màn hình hẹp: xem theo ngày */
const view = typeof window !== 'undefined' && window.matchMedia?.('(max-width: 640px)').matches ? 'day' : 'week';
const events = computed(() => (sessions.value ? toTimetableEvents(sessions.value) : null));
let seq = 0;

async function load(range = el.value?.range) {
  if (!range) return;
  const my = ++seq;
  error.value = '';
  try {
    const list = await props.fetcher({ from: range.from.toISOString(), to: range.to.toISOString() });
    if (my !== seq) return;
    sessions.value = list;
    emit('loaded', list);
  } catch (e) {
    if (my === seq) error.value = e.message;
  }
}

function onRange(e) {
  load(e.detail);
}

onMounted(async () => {
  await nextTick();
  load();
});

defineExpose({ reload: () => load() });
</script>

<style scoped>
.schedule-calendar {
  display: grid;
  gap: 8px;
}
.legend {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 16px;
  margin: 0;
}
.dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 3px;
  vertical-align: -1px;
  margin-right: 4px;
}
.dot.primary {
  background: var(--hui-primary);
}
.dot.success {
  background: var(--hui-success);
}
.dot.secondary {
  background: var(--hui-secondary);
}
.dot.cancelled {
  background: transparent;
  border: 1px dashed var(--hui-text-muted, #999);
}
</style>

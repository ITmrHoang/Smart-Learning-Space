<template>
  <hui-modal :open="open" :title="offer ? `Đặt lịch: ${offer.title}` : 'Đặt lịch học kèm'" size="lg" ok-text="Gửi yêu cầu" cancel-text="Hủy" :confirmLoading="saving" @hui-ok="onOk" @hui-close="close">
    <div v-if="offer" class="stack">
      <p class="small" style="margin: 0">
        Giáo viên <b>{{ offer.teacherName }}</b> · {{ formatDuration(offer.durationMinutes) }} / buổi.
        Chọn một giờ còn trống, giáo viên nhận thì buổi học hiện trong "Lịch học" kèm link vào học.
      </p>
      <p v-if="offer.description" class="muted small" style="margin: 0">{{ offer.description }}</p>
      <hui-alert v-if="error" severity="error" :description="error" />
      <p v-if="loading" class="muted small">Đang tải giờ trống…</p>
      <p v-else-if="!days.length" class="panel empty muted">Hiện không còn giờ trống. Hãy quay lại sau.</p>
      <div v-for="d in days" :key="d.date" class="day">
        <b class="small">{{ d.label }}</b>
        <div class="slots">
          <button
            v-for="s in d.slots"
            :key="s.startsAt"
            type="button"
            class="slot"
            :class="{ active: picked === s.startsAt }"
            :aria-pressed="picked === s.startsAt"
            @click="picked = s.startsAt"
          >
            {{ s.time }}
          </button>
        </div>
      </div>
      <label class="stack small">
        Em muốn học gì? (giáo viên đọc trước khi nhận)
        <textarea v-model="note" class="native-input" rows="2" maxlength="1000" placeholder="Ví dụ: Em chưa hiểu cách giải hệ phương trình bằng phương pháp thế" />
      </label>
    </div>
  </hui-modal>
</template>

<script setup>
/** Học sinh chọn giờ trống của một khung học kèm rồi gửi yêu cầu. */
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, ref, watch } from 'vue';

import { tutoringApi } from '../../api';
import { formatDuration, groupSlotsByDay } from '../../utils/schedule';

const props = defineProps({
  open: { type: Boolean, default: false },
  /** OfferView */
  offer: { type: Object, default: null },
});
const emit = defineEmits(['update:open', 'booked']);
const slots = ref([]);
const loading = ref(false);
const saving = ref(false);
const error = ref('');
const picked = ref('');
const note = ref('');
const days = computed(() => groupSlotsByDay(slots.value));

async function load() {
  loading.value = true;
  error.value = '';
  try {
    slots.value = await tutoringApi.slots(props.offer.id);
  } catch (e) {
    error.value = e.message;
  } finally {
    loading.value = false;
  }
}

watch(
  () => props.open,
  (open) => {
    if (!open || !props.offer) return;
    picked.value = '';
    note.value = '';
    load();
  },
);

async function onOk(e) {
  e.preventDefault();
  if (!picked.value) {
    error.value = 'Hãy chọn một giờ còn trống.';
    return;
  }
  saving.value = true;
  error.value = '';
  try {
    const booking = await tutoringApi.book(props.offer.id, picked.value, note.value.trim());
    message.success('Đã gửi yêu cầu. Bạn sẽ nhận thông báo khi giáo viên trả lời.');
    emit('booked', booking);
    close();
  } catch (err) {
    error.value = err.message;
    if (err.code === 'SLOT_TAKEN') load();
  } finally {
    saving.value = false;
  }
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.day {
  display: grid;
  gap: 6px;
}
.slots {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}
.slot {
  min-width: 64px;
  padding: 6px 10px;
  border: 1px solid var(--hui-border, #d9d9d9);
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-bg-container, #fff);
  font: inherit;
  cursor: pointer;
}
.slot:hover {
  border-color: var(--hui-primary, #0864ad);
}
.slot.active {
  background: var(--hui-primary, #0864ad);
  border-color: var(--hui-primary, #0864ad);
  color: #fff;
}
textarea.native-input {
  font: inherit;
  resize: vertical;
}
</style>

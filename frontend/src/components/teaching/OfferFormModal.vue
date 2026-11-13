<template>
  <hui-modal :open="open" :title="offer ? 'Sửa lịch học kèm' : 'Mở lịch học kèm 1-1'" size="lg" ok-text="Lưu" cancel-text="Hủy" :confirmLoading="loading" @hui-ok="onOk" @hui-close="close">
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-input v-model="form.title" label="Tên" placeholder="Ví dụ: Học kèm Toán 9 — ôn thi vào 10" :error="fields.title" required />
      <hui-select label="Môn học (không bắt buộc)" :options="subjectOptions" :value="form.subjectId" clearable @change="form.subjectId = $event.detail.value || null" />
      <label class="stack small">
        Mô tả
        <textarea v-model="form.description" class="native-input" rows="2" maxlength="2000" placeholder="Học gì, cho ai, cần chuẩn bị gì" />
      </label>
      <div class="row">
        <label class="stack small">
          Một buổi
          <select v-model.number="form.durationMinutes" class="native-select">
            <option v-for="d in DURATIONS" :key="d.value" :value="d.value">{{ d.label }}</option>
          </select>
        </label>
        <label class="stack small">
          Đặt trước ít nhất
          <select v-model.number="form.minNoticeHours" class="native-select">
            <option v-for="h in [0, 2, 6, 12, 24, 48]" :key="h" :value="h">{{ h ? `${h} giờ` : 'Không cần' }}</option>
          </select>
        </label>
        <label class="stack small">
          Cho đặt trước tối đa
          <select v-model.number="form.horizonDays" class="native-select">
            <option v-for="d in [7, 14, 21, 30, 60]" :key="d" :value="d">{{ d }} ngày</option>
          </select>
        </label>
      </div>

      <div class="stack">
        <b class="small">Khung giờ rảnh hằng tuần (giờ Việt Nam)</b>
        <div v-for="(w, i) in form.windows" :key="i" class="row window">
          <select v-model.number="w.weekday" class="native-select">
            <option v-for="d in WEEKDAY_NAMES" :key="d.value" :value="d.value">{{ d.label }}</option>
          </select>
          <input v-model="w.start" type="time" class="native-input" />
          <span>–</span>
          <input v-model="w.end" type="time" class="native-input" />
          <button type="button" class="remove" title="Bỏ khung giờ" @click="form.windows.splice(i, 1)">✕</button>
        </div>
        <div class="row">
          <hui-button size="sm" variant="outline" @hui-click="addWindow">+ Khung giờ</hui-button>
          <span class="muted small">Học sinh chọn một khoảng {{ formatDuration(form.durationMinutes) }} trong các khung này; giờ đã có buổi học khác tự bị ẩn.</span>
        </div>
        <p v-if="fields.windows" class="field-error">{{ fields.windows }}</p>
      </div>

      <hui-input v-model="form.meetingUrl" label="Link họp (Google Meet / Zoom…)" placeholder="https://meet.google.com/..." :error="fields.meetingUrl" />
      <label class="stack small">
        Ai được đặt lịch
        <select v-model="form.audience" class="native-select">
          <option value="my_students">Học sinh các lớp học của tôi</option>
          <option value="everyone">Mọi tài khoản (hiện trên trang giáo viên công khai)</option>
        </select>
      </label>
      <label class="small"><input v-model="form.active" type="checkbox" /> Đang mở cho đặt lịch</label>
    </form>
  </hui-modal>
</template>

<script setup>
import { computed, reactive, watch } from 'vue';

import { teachingApi } from '../../api';
import { useSubmit } from '../../utils/form';
import { useMasterData } from '../../utils/masterData';
import { formatDuration } from '../../utils/schedule';

const WEEKDAY_NAMES = [
  { value: 1, label: 'Thứ Hai' },
  { value: 2, label: 'Thứ Ba' },
  { value: 3, label: 'Thứ Tư' },
  { value: 4, label: 'Thứ Năm' },
  { value: 5, label: 'Thứ Sáu' },
  { value: 6, label: 'Thứ Bảy' },
  { value: 7, label: 'Chủ nhật' },
];
const DURATIONS = [30, 45, 60, 90, 120].map((m) => ({ value: m, label: formatDuration(m) }));

const props = defineProps({
  open: { type: Boolean, default: false },
  /** OfferView khi sửa */
  offer: { type: Object, default: null },
});
const emit = defineEmits(['update:open', 'saved']);
const { data: master, load: loadMaster } = useMasterData();
const { loading, error, fields, run, reset } = useSubmit();
const form = reactive({});
const subjectOptions = computed(() => (master.value?.subjects ?? []).map((s) => ({ value: s.id, label: s.name })));

function blank() {
  return {
    title: '',
    subjectId: null,
    description: '',
    durationMinutes: 60,
    minNoticeHours: 12,
    horizonDays: 14,
    windows: [{ weekday: 1, start: '19:00', end: '21:00' }],
    meetingUrl: '',
    audience: 'my_students',
    active: true,
  };
}

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    loadMaster();
    reset();
    const o = props.offer;
    Object.assign(
      form,
      o
        ? {
            title: o.title,
            subjectId: o.subjectId,
            description: o.description,
            durationMinutes: o.durationMinutes,
            minNoticeHours: o.minNoticeHours,
            horizonDays: o.horizonDays,
            windows: o.windows.map((w) => ({ ...w })),
            meetingUrl: o.meetingUrl ?? '',
            audience: o.audience,
            active: o.active,
          }
        : blank(),
    );
  },
);

function addWindow() {
  const last = form.windows[form.windows.length - 1];
  form.windows.push({ weekday: last ? (last.weekday % 7) + 1 : 1, start: last?.start ?? '19:00', end: last?.end ?? '21:00' });
}

async function save() {
  const body = { ...form, title: form.title.trim(), meetingUrl: form.meetingUrl.trim(), windows: form.windows.map((w) => ({ ...w })) };
  const res = await run(() => (props.offer ? teachingApi.updateOffer(props.offer.id, body) : teachingApi.createOffer(body)));
  if (res.ok) {
    emit('saved', res.data);
    close();
  }
}

function onOk(e) {
  e.preventDefault();
  save();
}

function close() {
  emit('update:open', false);
}
</script>

<style scoped>
.window input[type='time'] {
  width: 120px;
}
.remove {
  border: 0;
  background: none;
  cursor: pointer;
  color: var(--hui-text-secondary, #6b7280);
}
textarea.native-input {
  font: inherit;
  resize: vertical;
}
</style>

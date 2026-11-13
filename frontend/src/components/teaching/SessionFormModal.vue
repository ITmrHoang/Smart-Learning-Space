<template>
  <hui-modal :open="open" :title="isEdit ? 'Sửa buổi học' : 'Tạo buổi học'" size="lg" ok-text="Lưu" cancel-text="Hủy" :confirmLoading="loading" @hui-ok="onOk" @hui-close="close">
    <form class="form-grid" novalidate @submit.prevent="save">
      <hui-alert v-if="error" severity="error" :description="error" />
      <hui-alert v-for="(msg, key) in otherErrors" :key="key" severity="error" :description="msg" />

      <div v-if="isEdit && session?.scheduleId" class="scope">
        <span class="field-label">Áp dụng cho</span>
        <hui-radio-group :options="scopeOptions" :value="form.scope" @change="form.scope = $event.detail.value" />
        <p v-if="form.scope === 'following'" class="muted small">Các buổi sau của cùng lịch lặp được dời cùng một khoảng thời gian và nhận nội dung mới.</p>
      </div>

      <hui-input v-model="form.title" label="Tên buổi học" placeholder="Ví dụ: Toán 9 — Hệ phương trình (buổi 3)" :error="fields.title" required />

      <div class="grid-3">
        <label class="field">
          <span class="field-label">Ngày</span>
          <hui-date-picker :value="form.date" format="DD/MM/YYYY" :status="fields.startsAt ? 'error' : ''" label="Ngày học" @change="onDate($event.detail.value)" />
        </label>
        <label class="field">
          <span class="field-label">Giờ bắt đầu</span>
          <hui-time-picker :value="form.time" format="HH:mm" :minuteStep="5" label="Giờ bắt đầu" @change="form.time = $event.detail.value" />
        </label>
        <hui-select label="Thời lượng" :options="durationOptions" :value="form.duration" :error="fields.durationMinutes" @change="form.duration = $event.detail.value" />
      </div>
      <p v-if="fields.startsAt" class="field-error">{{ fields.startsAt }}</p>
      <p class="muted small">{{ whenText }}</p>

      <div v-if="!isEdit" class="repeat">
        <hui-switch :checked="form.repeat" label="Lặp lại hằng tuần" @change="form.repeat = $event.detail.checked" />
        <template v-if="form.repeat">
          <div class="field">
            <span class="field-label">Vào các thứ</span>
            <hui-checkbox-group :options="weekdayOptions" :value="form.weekdays" @change="form.weekdays = $event.detail.value" />
            <span v-if="fields.weekdays" class="field-error">{{ fields.weekdays }}</span>
          </div>
          <label class="field">
            <span class="field-label">Đến hết ngày</span>
            <hui-date-picker :value="form.until" format="DD/MM/YYYY" :min="form.date" label="Ngày kết thúc lịch lặp" @change="form.until = $event.detail.value" />
          </label>
          <p class="muted small">Tối đa 200 buổi. Mỗi buổi là một buổi riêng: sửa / hủy được từng buổi hoặc từ một buổi trở đi.</p>
        </template>
      </div>

      <div class="grid-2">
        <hui-select label="Thuộc lớp học" :options="classOptions" :value="form.classId" placeholder="Không thuộc lớp học nào" clearable @change="onClass($event.detail.value)" />
        <hui-select label="Ai được vào" :options="accessOptions" :value="form.access" @change="form.access = $event.detail.value" />
      </div>

      <fieldset class="invitees">
        <legend>Mời</legend>
        <div class="grid-3">
          <hui-select label="Lớp học" mode="multiple" placeholder="Chọn lớp học" :options="classOptions" :value="form.classIds" searchable @change="form.classIds = $event.detail.value" />
          <hui-select label="Nhóm học sinh" mode="multiple" placeholder="Chọn nhóm" :options="groupOptions" :value="form.groupIds" searchable @change="form.groupIds = $event.detail.value" />
          <hui-select label="Học sinh" mode="multiple" placeholder="Chọn học sinh" :options="studentOptions" :value="form.userIds" searchable :maxTagCount="3" @change="form.userIds = $event.detail.value" />
        </div>
        <p class="muted small">
          Mời theo lớp học / nhóm: học sinh vào lớp học hay nhóm sau này cũng thấy buổi. Ai được mời nhận thông báo và email kèm lịch (.ics).
          <RouterLink :to="{ name: 'teach-groups' }" target="_blank">Quản lý nhóm học sinh</RouterLink>
        </p>
      </fieldset>

      <div class="grid-2">
        <div class="stack room-choice">
          <span class="small"><b>Phòng học</b></span>
          <label class="small"><input v-model="form.room" type="radio" value="live" :disabled="!liveEnabled" /> Phòng học của hệ thống (video, bảng thông minh, giơ tay){{ liveEnabled ? '' : ' — chưa cấu hình' }}</label>
          <label class="small"><input v-model="form.room" type="radio" value="external" /> Link họp ngoài (Google Meet, Zoom…)</label>
          <hui-input v-if="form.room === 'external'" v-model="form.meetingUrl" label="Link phòng học" placeholder="https://meet.google.com/…" :error="fields.meetingUrl" />
          <p v-if="fields.room" class="field-error">{{ fields.room }}</p>
        </div>
        <hui-select label="Nhắc học sinh" :options="remindOptions" :value="form.remindMinutes" @change="form.remindMinutes = $event.detail.value" />
      </div>
      <hui-select label="Bài sẽ dạy" mode="multiple" placeholder="Chọn bài học (không bắt buộc)" :options="lessonOptions" :value="form.lessonIds" searchable @change="form.lessonIds = $event.detail.value" />
      <hui-textarea label="Ghi chú cho học sinh" :value="form.description" :rows="3" placeholder="Chuẩn bị gì trước buổi học…" :error="fields.description" @input="form.description = $event.detail.value" />
    </form>
  </hui-modal>
</template>

<script setup>
import { message } from '@itmrhoang/h-ui/hui-message';
import { computed, reactive, ref, watch } from 'vue';

import { liveApi, teachingApi } from '../../api';
import { useSubmit } from '../../utils/form';
import {
  ACCESS_LABELS,
  DURATION_OPTIONS,
  REMIND_OPTIONS,
  WEEKDAYS,
  addDaysISO,
  formatDuration,
  formatSessionTime,
  fromLocal,
  isoWeekday,
  localParts,
} from '../../utils/schedule';
import { toOptions } from '../../utils/teaching';

const props = defineProps({
  open: { type: Boolean, default: false },
  /** SessionDetail (giáo viên xem) khi sửa */
  session: { type: Object, default: null },
  /** Gợi ý khi tạo: { start: Date, end: Date, classId } */
  preset: { type: Object, default: null },
});
const emit = defineEmits(['update:open', 'saved']);
const { loading, error, fields, run, reset } = useSubmit();

const classes = ref([]);
const groups = ref([]);
const students = ref([]);
const lessons = ref([]);
const form = reactive({});

const isEdit = computed(() => Boolean(props.session));
const SHOWN = ['title', 'startsAt', 'durationMinutes', 'meetingUrl', 'weekdays', 'description', 'room'];
const otherErrors = computed(() => Object.fromEntries(Object.entries(fields).filter(([k]) => !SHOWN.includes(k))));

const str = (o) => o.map((x) => ({ value: String(x.value), label: x.label }));
const weekdayOptions = str(WEEKDAYS);
const remindOptions = str(REMIND_OPTIONS);
/** "Khách có link" chỉ dùng với phòng của hệ thống (có phòng chờ). */
const accessOptions = computed(() =>
  toOptions(ACCESS_LABELS).filter((o) => o.value !== 'public_link' || form.room === 'live'),
);
const liveEnabled = ref(false);
const scopeOptions = [
  { value: 'this', label: 'Chỉ buổi này' },
  { value: 'following', label: 'Buổi này và các buổi sau' },
];
const durationOptions = computed(() => {
  const list = str(DURATION_OPTIONS);
  if (form.duration && !list.some((o) => o.value === form.duration)) list.push({ value: form.duration, label: formatDuration(form.duration) });
  return list.sort((a, b) => a.value - b.value);
});
const classOptions = computed(() => classes.value.map((c) => ({ value: c.id, label: c.title })));
const groupOptions = computed(() => groups.value.map((g) => ({ value: g.id, label: `${g.name} (${g.memberCount})` })));
const lessonOptions = computed(() => lessons.value.map((l) => ({ value: l.id, label: l.title })));
const studentOptions = computed(() => {
  const list = students.value.map((s) => ({ value: s.id, label: `${s.fullName} (@${s.username})` }));
  // Học sinh đã mời nhưng không còn trong lớp học nào của giáo viên vẫn hiện tên
  for (const u of props.session?.invitees?.users ?? []) if (!list.some((o) => o.value === u.id)) list.push({ value: u.id, label: u.name });
  return list;
});

const whenText = computed(() => {
  const start = fromLocal(form.date, form.time);
  if (!start) return 'Chọn ngày và giờ bắt đầu.';
  const end = new Date(new Date(start).getTime() + Number(form.duration) * 60_000);
  return `Buổi học: ${formatSessionTime(start, end)}${form.repeat && !isEdit.value ? ' (buổi đầu tiên rơi vào thứ đã chọn từ ngày này)' : ''}.`;
});

async function loadOptions() {
  const [cs, gs, ss, ls] = await Promise.all([
    teachingApi.classes({ pageSize: 100 }).catch(() => ({ items: [] })),
    teachingApi.groups().catch(() => []),
    teachingApi.students({}).catch(() => []),
    teachingApi.lessons({ pageSize: 100 }).catch(() => ({ items: [] })),
  ]);
  classes.value = cs.items;
  groups.value = gs;
  students.value = ss;
  lessons.value = ls.items;
}

function fill() {
  const s = props.session;
  if (s) {
    const { date, time } = localParts(s.startsAt);
    Object.assign(form, {
      title: s.title,
      classId: s.classId,
      date,
      time,
      duration: String(s.durationMinutes),
      repeat: false,
      weekdays: [String(isoWeekday(date))],
      until: date,
      classIds: (s.invitees?.classes ?? []).map((x) => x.id),
      groupIds: (s.invitees?.groups ?? []).map((x) => x.id),
      userIds: (s.invitees?.users ?? []).map((x) => x.id),
      access: s.access,
      meetingUrl: s.meetingUrl ?? '',
      room: s.room ?? 'external',
      remindMinutes: String(s.remindMinutes),
      lessonIds: s.lessons.map((l) => l.id),
      description: s.description ?? '',
      scope: 'this',
    });
    return;
  }
  const p = props.preset ?? {};
  // Mặc định: giờ tròn kế tiếp
  const start = p.start ? new Date(p.start) : new Date(Math.ceil(Date.now() / 3_600_000) * 3_600_000);
  const minutes = p.start && p.end ? Math.round((new Date(p.end) - start) / 60_000) : 0;
  const { date, time } = localParts(start);
  Object.assign(form, {
    title: '',
    classId: p.classId ?? null,
    date,
    time,
    duration: String(minutes >= 60 ? minutes : 90),
    repeat: false,
    weekdays: [String(isoWeekday(date))],
    until: addDaysISO(date, 7 * 8 - 1),
    classIds: p.classId ? [p.classId] : [],
    groupIds: [],
    userIds: [],
    access: 'invited',
    meetingUrl: '',
    room: liveEnabled.value ? 'live' : 'external',
    remindMinutes: '60',
    lessonIds: [],
    description: '',
    scope: 'this',
  });
}

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    reset();
    // Có phòng của hệ thống (LiveKit) thì buổi mới mặc định dùng phòng đó.
    liveEnabled.value = (await liveApi.status().catch(() => ({ enabled: false }))).enabled;
    fill();
    await loadOptions();
  },
);

/** Đổi ngày: lịch lặp đang chỉ có thứ của ngày cũ → theo thứ của ngày mới. */
function onDate(date) {
  const old = String(isoWeekday(form.date));
  form.date = date;
  if (form.weekdays.length === 1 && form.weekdays[0] === old && date) form.weekdays = [String(isoWeekday(date))];
  if (form.until < date) form.until = addDaysISO(date, 7 * 8 - 1);
}

/** Chọn lớp học của buổi: chưa mời ai thì mời luôn cả lớp học. */
function onClass(id) {
  form.classId = id || null;
  if (id && !form.classIds.length && !form.groupIds.length && !form.userIds.length) form.classIds = [id];
}

function payload() {
  return {
    title: form.title.trim(),
    description: form.description,
    classId: form.classId,
    startsAt: fromLocal(form.date, form.time),
    durationMinutes: Number(form.duration),
    invitees: { classIds: form.classIds, groupIds: form.groupIds, userIds: form.userIds },
    access: form.room !== 'live' && form.access === 'public_link' ? 'invited' : form.access,
    room: form.room,
    meetingUrl: form.room === 'external' ? form.meetingUrl.trim() || null : null,
    remindMinutes: Number(form.remindMinutes),
    lessonIds: form.lessonIds,
  };
}

async function save() {
  if (!fromLocal(form.date, form.time)) {
    fields.startsAt = 'Chọn ngày và giờ bắt đầu.';
    return;
  }
  const body = payload();
  const res = await run(() =>
    isEdit.value
      ? teachingApi.updateSession(props.session.id, { ...body, scope: form.scope })
      : teachingApi.createSession({
        ...body,
        repeat: form.repeat ? { weekdays: form.weekdays.map(Number).sort(), until: form.until } : null,
      }),
  );
  if (!res.ok) return;
  const d = res.data;
  if (isEdit.value) message.success(d.updated > 1 ? `Đã cập nhật ${d.updated} buổi học. Học sinh được báo lại giờ mới.` : 'Đã cập nhật buổi học.');
  else message.success(d.sessions.length > 1 ? `Đã tạo ${d.sessions.length} buổi học.` : 'Đã tạo buổi học, đã gửi lời mời.');
  if (d.studentConflicts > 0) message.warning(`${d.studentConflicts} học sinh được mời đang có buổi học khác trùng giờ.`);
  emit('saved', d);
  close();
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
.field {
  display: grid;
  gap: 6px;
  align-content: start;
}
.field-label {
  font-size: 0.875rem;
  font-weight: 500;
}
.field hui-date-picker,
.field hui-time-picker {
  width: 100%;
}
.repeat,
.scope {
  display: grid;
  gap: 10px;
  padding: 12px;
  border-radius: var(--hui-radius-md, 8px);
  background: var(--hui-fill-tertiary, #f5f6f8);
}
.invitees {
  border: 1px solid var(--hui-border-secondary, #eceef1);
  border-radius: var(--hui-radius-md, 8px);
  padding: 8px 12px 12px;
  margin: 0;
  display: grid;
  gap: 8px;
}
.invitees legend {
  padding: 0 6px;
  font-weight: 600;
}
</style>

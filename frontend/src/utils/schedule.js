/**
 * Lịch & buổi học: nhãn tiếng Việt, đổi giờ, trạng thái buổi, dữ liệu cho hui-timetable.
 * API trả giờ UTC (ISO có Z); giao diện hiển thị theo giờ của máy (người dùng ở Việt Nam = UTC+7,
 * trùng với giờ backend dùng cho lịch lặp và email).
 */

export const ACCESS_LABELS = {
  invited: 'Chỉ người được mời',
  members: 'Mọi thành viên lớp học',
  logged_in: 'Mọi tài khoản đã đăng nhập',
  public_link: 'Khách có link (phòng chờ, giáo viên duyệt)',
};

export const RSVP_LABELS = { yes: 'Tham gia', no: 'Không tham gia' };
export const RSVP_COLORS = { yes: 'success', no: 'secondary' };

/** 1 = thứ Hai … 7 = Chủ nhật (giống backend). */
export const WEEKDAYS = [
  { value: 1, label: 'T2' },
  { value: 2, label: 'T3' },
  { value: 3, label: 'T4' },
  { value: 4, label: 'T5' },
  { value: 5, label: 'T6' },
  { value: 6, label: 'T7' },
  { value: 7, label: 'CN' },
];

const WEEKDAY_NAMES = ['Chủ nhật', 'thứ Hai', 'thứ Ba', 'thứ Tư', 'thứ Năm', 'thứ Sáu', 'thứ Bảy'];

/** Mở nút "Vào học" trước giờ học (phút) — khớp backend JOIN_EARLY. */
export const JOIN_EARLY_MINUTES = 10;

export const PHASE_LABELS = {
  cancelled: 'Đã hủy',
  live: 'Đang diễn ra',
  soon: 'Sắp bắt đầu',
  upcoming: 'Sắp tới',
  ended: 'Đã kết thúc',
};
export const PHASE_COLORS = { cancelled: 'danger', live: 'success', soon: 'warning', upcoming: 'info', ended: 'secondary' };

export const NOTIFICATION_KIND_LABELS = {
  session_invite: 'Lời mời buổi học',
  session_updated: 'Buổi học thay đổi',
  session_cancelled: 'Buổi học bị hủy',
  session_reminder: 'Nhắc lịch',
  enrollment_approved: 'Được duyệt vào lớp',
  class_added: 'Được thêm vào lớp',
  path_published: 'Lộ trình mới',
  question_new: 'Câu hỏi mới',
  question_reply: 'Trả lời mới',
  content_reported: 'Báo vi phạm',
  homework_submitted: 'Bài nộp mới',
  homework_graded: 'Bài đã chấm',
  homework_returned: 'Bài cần làm lại',
  homework_due: 'Sắp hết hạn nộp',
  booking_requested: 'Yêu cầu học kèm',
  booking_approved: 'Lịch học kèm được nhận',
  booking_declined: 'Lịch học kèm bị từ chối',
  booking_cancelled: 'Lịch học kèm bị hủy',
  assistant_added: 'Mời làm trợ giảng',
};
export const NOTIFICATION_KIND_ICONS = {
  session_invite: 'calendar',
  session_updated: 'edit',
  session_cancelled: 'error',
  session_reminder: 'clock',
  enrollment_approved: 'success',
  class_added: 'user',
  path_published: 'star',
  question_new: 'chat',
  question_reply: 'chat',
  content_reported: 'warning',
  homework_submitted: 'edit',
  homework_graded: 'success',
  homework_returned: 'warning',
  homework_due: 'clock',
  booking_requested: 'calendar',
  booking_approved: 'success',
  booking_declined: 'error',
  booking_cancelled: 'error',
  assistant_added: 'user',
};

/** 90 → "1 giờ 30 phút"; 45 → "45 phút"; 120 → "2 giờ". */
export function formatDuration(minutes) {
  const m = Math.max(0, Math.round(Number(minutes) || 0));
  const h = Math.floor(m / 60);
  const r = m % 60;
  if (!h) return `${r} phút`;
  return r ? `${h} giờ ${r} phút` : `${h} giờ`;
}

export const DURATION_OPTIONS = [30, 45, 60, 90, 120, 150, 180, 240].map((m) => ({ value: m, label: formatDuration(m) }));

export const REMIND_OPTIONS = [
  { value: 0, label: 'Không nhắc' },
  { value: 15, label: '15 phút trước' },
  { value: 30, label: '30 phút trước' },
  { value: 60, label: '1 giờ trước' },
  { value: 120, label: '2 giờ trước' },
  { value: 1440, label: '1 ngày trước' },
];

const pad = (n) => String(n).padStart(2, '0');

/** Date | ISO → { date: 'YYYY-MM-DD', time: 'HH:mm' } theo giờ của máy. */
export function localParts(value) {
  const d = new Date(value);
  return {
    date: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`,
    time: `${pad(d.getHours())}:${pad(d.getMinutes())}`,
  };
}

/** Ngày + giờ theo giờ của máy → ISO UTC (gửi lên API). null nếu thiếu / sai. */
export function fromLocal(date, time) {
  if (!date || !time) return null;
  const d = new Date(`${date}T${time}`);
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

/** 'YYYY-MM-DD' → thứ trong tuần 1 (thứ Hai) … 7 (Chủ nhật). */
export function isoWeekday(date) {
  const [y, m, d] = String(date).split('-').map(Number);
  const day = new Date(y, m - 1, d).getDay();
  return day === 0 ? 7 : day;
}

/** Cộng ngày cho 'YYYY-MM-DD'. */
export function addDaysISO(date, n) {
  const [y, m, d] = String(date).split('-').map(Number);
  return localParts(new Date(y, m - 1, d + n)).date;
}

/** "thứ Ba, 10/11/2026" */
export function formatDay(value) {
  const d = new Date(value);
  return `${WEEKDAY_NAMES[d.getDay()]}, ${pad(d.getDate())}/${pad(d.getMonth() + 1)}/${d.getFullYear()}`;
}

/** "19:30 – 21:00, thứ Ba 10/11/2026" (giờ kết thúc khác ngày thì ghi đủ ngày). */
export function formatSessionTime(startsAt, endsAt) {
  if (!startsAt) return '—';
  const s = new Date(startsAt);
  const e = endsAt ? new Date(endsAt) : s;
  const day = `${WEEKDAY_NAMES[s.getDay()]} ${pad(s.getDate())}/${pad(s.getMonth() + 1)}/${s.getFullYear()}`;
  const start = `${pad(s.getHours())}:${pad(s.getMinutes())}`;
  const sameDay = s.toDateString() === e.toDateString();
  const end = `${pad(e.getHours())}:${pad(e.getMinutes())}${sameDay ? '' : ` ${pad(e.getDate())}/${pad(e.getMonth() + 1)}`}`;
  return e - s > 0 ? `${start} – ${end}, ${day}` : `${start}, ${day}`;
}

/**
 * Trạng thái buổi theo thời điểm `now`:
 * cancelled · live (đang học) · soon (đã mở "Vào học", trước giờ học ≤ 10 phút) · upcoming · ended.
 */
export function sessionPhase(session, now = Date.now()) {
  if (session.status === 'cancelled') return 'cancelled';
  const start = new Date(session.startsAt).getTime();
  const end = new Date(session.endsAt).getTime();
  const t = new Date(now).getTime();
  if (t > end) return 'ended';
  if (t >= start) return 'live';
  if (t >= start - JOIN_EARLY_MINUTES * 60_000) return 'soon';
  return 'upcoming';
}

/** "còn 25 phút" / "còn 3 giờ" / "còn 2 ngày" / "" khi đã bắt đầu. */
export function timeUntil(startsAt, now = Date.now()) {
  const ms = new Date(startsAt).getTime() - new Date(now).getTime();
  if (ms <= 0) return '';
  const min = Math.ceil(ms / 60_000);
  if (min < 60) return `còn ${min} phút`;
  const h = Math.floor(min / 60);
  if (h < 24) return `còn ${h} giờ`;
  return `còn ${Math.floor(h / 24)} ngày`;
}

/**
 * SessionSummary[] → sự kiện của hui-timetable.
 * Màu: buổi mình dạy = primary, được mời = success, đã báo không tham gia = secondary; buổi hủy gạch ngang.
 */
export function toTimetableEvents(sessions) {
  return (sessions ?? []).map((s) => ({
    id: s.id,
    title: s.title,
    start: s.startsAt,
    end: s.endsAt,
    subtitle: s.isOwner ? s.classTitle ?? '' : [s.classTitle, s.teacherName].filter(Boolean).join(' · '),
    color: s.isOwner ? 'primary' : s.myRsvp === 'no' ? 'secondary' : 'success',
    status: s.status,
  }));
}

/** Gộp nhiều danh sách buổi (vd buổi dạy + buổi học của giáo viên), bỏ trùng, sắp theo giờ. */
export function mergeSessions(...lists) {
  const map = new Map();
  for (const list of lists) for (const s of list ?? []) if (!map.has(s.id)) map.set(s.id, s);
  return [...map.values()].sort((a, b) => new Date(a.startsAt) - new Date(b.startsAt));
}

/** Khoảng học kèm còn trống → nhóm theo ngày (giờ của máy): [{ date, label, slots }]. */
export function groupSlotsByDay(slots) {
  const days = new Map();
  for (const s of slots) {
    const { date, time } = localParts(s.startsAt);
    if (!days.has(date)) days.set(date, { date, label: formatDay(s.startsAt), slots: [] });
    days.get(date).slots.push({ ...s, time });
  }
  return [...days.values()];
}

/** Khung giờ hằng tuần của lịch học kèm → "T2 19:00–21:00, T4 14:00–16:00". */
export function formatWindows(windows) {
  const label = (d) => WEEKDAYS.find((w) => w.value === d)?.label ?? '';
  return [...(windows ?? [])]
    .sort((a, b) => a.weekday - b.weekday || a.start.localeCompare(b.start))
    .map((w) => `${label(w.weekday)} ${w.start}–${w.end}`)
    .join(', ');
}

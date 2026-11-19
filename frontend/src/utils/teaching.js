/** Nhãn tiếng Việt + hàm tiện ích cho phần dạy / học (bài học, lớp học, câu hỏi, đề). */

export const QUESTION_KIND_LABELS = {
  single: 'Một đáp án',
  multiple: 'Nhiều đáp án',
  true_false: 'Đúng / sai',
  short: 'Trả lời ngắn',
  essay: 'Tự luận',
};

/** Trả lời ngắn: cách so đáp án. */
export const MATCH_MODE_LABELS = {
  auto: 'Số hoặc chữ (0,5 = 1/2)',
  expression: 'Biểu thức tương đương (2(x+1) = 2x + 2)',
};

/** Tổng điểm các bước của hướng dẫn chấm. */
export const rubricTotal = (rubric) => Math.round((rubric ?? []).reduce((s, r) => s + Number(r.points || 0), 0) * 100) / 100;

/** Điểm câu cố định trong đề: điểm riêng → tổng bước chấm (tự luận) → điểm mỗi câu của phần (giống server). */
export function questionPoints(section, question) {
  const custom = section.questionPoints?.[question.id];
  if (custom !== undefined && custom !== null && custom !== '') return Number(custom);
  const total = question.kind === 'essay' ? rubricTotal(question.rubric) : 0;
  return total > 0 ? total : Number(section.pointsPerQuestion);
}

export const LEVEL_LABELS = {
  nhan_biet: 'Nhận biết',
  thong_hieu: 'Thông hiểu',
  van_dung: 'Vận dụng',
  van_dung_cao: 'Vận dụng cao',
};
export const LEVEL_COLORS = { nhan_biet: 'success', thong_hieu: 'info', van_dung: 'warning', van_dung_cao: 'danger' };

export const VISIBILITY_LABELS = { public: 'Công khai', unlisted: 'Có link mới thấy', private: 'Riêng tư' };
export const ENROLLMENT_LABELS = { open: 'Vào học ngay', approval: 'Giáo viên duyệt', invite: 'Chỉ qua mã mời' };
export const MODE_LABELS = { self_paced: 'Tự học', live: 'Có buổi live' };
export const MEMBER_STATUS_LABELS = {
  pending: 'Chờ duyệt',
  active: 'Đang học',
  rejected: 'Bị từ chối',
  left: 'Đã rời lớp',
  removed: 'Bị mời ra',
};
export const MEMBER_STATUS_COLORS = { pending: 'warning', active: 'success', rejected: 'danger', left: 'secondary', removed: 'danger' };

export const EXAM_KIND_LABELS = { practice: 'Ôn tập', test: 'Kiểm tra' };
export const EXAM_KIND_COLORS = { practice: 'info', test: 'primary' };
export const SHOW_ANSWERS_LABELS = {
  after_submit: 'Ngay sau khi nộp',
  after_close: 'Sau khi đề đóng',
  never: 'Không hiện đáp án',
};
export const SCORING_LABELS = {
  per_question: 'Đúng hết mới có điểm',
  thpt_true_false: 'Đúng / sai theo thang THPT (0,1 / 0,25 / 0,5 / 1)',
};
export const STEP_KIND_LABELS = {
  milestone: 'Mốc',
  lesson: 'Bài học',
  exam: 'Đề',
  session: 'Buổi học',
  deck: 'Thẻ ghi nhớ',
  homework: 'Bài tập',
};
/** Bài nộp: chưa nộp (null) · đã nộp · trả lại làm lại · đã chấm. */
export const HOMEWORK_STATUS_LABELS = { submitted: 'Đã nộp, chờ chấm', returned: 'Cần làm lại', graded: 'Đã chấm' };
export const HOMEWORK_STATUS_COLORS = { submitted: 'info', returned: 'warning', graded: 'success' };
export const BOOKING_STATUS_LABELS = { pending: 'Chờ giáo viên', approved: 'Đã nhận', declined: 'Bị từ chối', cancelled: 'Đã hủy' };
export const BOOKING_STATUS_COLORS = { pending: 'warning', approved: 'success', declined: 'danger', cancelled: 'secondary' };
export const STEP_STATUS_LABELS = { done: 'Đã xong', available: 'Đang mở', locked: 'Chưa mở' };
export const STEP_STATUS_COLORS = { done: 'success', available: 'info', locked: 'secondary' };

/** Chuyển object nhãn thành options cho hui-select. */
export const toOptions = (labels) => Object.entries(labels).map(([value, label]) => ({ value, label }));

/** Chữ cái thứ tự phương án: 0 → A. */
export const letter = (i) => String.fromCharCode(65 + i);

/** Chữ cái ý của câu đúng / sai: 0 → a. */
export const statementLetter = (i) => String.fromCharCode(97 + i);

/** 7.25 → "7,25"; null → "—". */
export function formatScore(value) {
  if (value === null || value === undefined) return '—';
  return Number(value).toLocaleString('vi-VN', { maximumFractionDigits: 2 });
}

/**
 * Link video → địa chỉ nhúng.
 * - YouTube (watch?v=, youtu.be/, shorts/, embed/) → iframe youtube-nocookie
 * - Link file (mp4, webm, m3u8…) → thẻ <video>
 * @returns {{ type: 'youtube' | 'file', src: string } | null}
 */
export function videoEmbed(url) {
  if (!url) return null;
  let u;
  try {
    u = new URL(url);
  } catch {
    return null;
  }
  if (!/^https?:$/.test(u.protocol)) return null;
  const host = u.hostname.replace(/^www\.|^m\./, '');
  let id = null;
  if (host === 'youtu.be') id = u.pathname.slice(1).split('/')[0];
  else if (host === 'youtube.com' || host === 'youtube-nocookie.com') {
    if (u.pathname === '/watch') id = u.searchParams.get('v');
    else {
      const m = /^\/(embed|shorts|live)\/([^/?#]+)/.exec(u.pathname);
      if (m) id = m[2];
    }
  }
  if (id !== null) {
    if (!/^[\w-]{6,20}$/.test(id)) return null;
    const start = Number.parseInt(u.searchParams.get('t') || u.searchParams.get('start') || '0', 10);
    return { type: 'youtube', src: `https://www.youtube-nocookie.com/embed/${id}${start > 0 ? `?start=${start}` : ''}` };
  }
  return { type: 'file', src: u.toString() };
}

/** Số giây còn lại tới hạn (tính theo giờ server để không lệch đồng hồ máy). */
export function secondsLeft(deadlineAt, serverNow, clientNowAtLoad, clientNow = Date.now()) {
  if (!deadlineAt) return null;
  const offset = new Date(serverNow).getTime() - clientNowAtLoad;
  const left = (new Date(deadlineAt).getTime() - (clientNow + offset)) / 1000;
  return Math.max(0, Math.floor(left));
}

/** 125 → "02:05", 3725 → "1:02:05". */
export function formatClock(total) {
  if (total === null || total === undefined) return '';
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const pad = (n) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

/** Câu đã có câu trả lời chưa (đếm câu đã làm). */
export function isAnswered(item) {
  const a = item.answer;
  if (!a) return false;
  if (item.kind === 'essay') return Boolean((a.text && a.text.trim()) || (a.files || []).length);
  if (item.kind === 'short') return Boolean(a.text && a.text.trim());
  if (item.kind === 'true_false') return Object.keys(a.truth || {}).length === item.options.length;
  return (a.selected || []).length > 0;
}

/** Câu hỏi của giáo viên → dạng câu trong bài làm, kèm đáp án (để xem bằng QuestionView). */
export function questionAsItem(q, { withContent = false } = {}) {
  return {
    position: 0,
    kind: q.kind,
    content: withContent ? q.content : '',
    points: 1,
    options: q.options,
    answer: null,
    checked: true,
    manual: q.kind === 'essay',
    files: [],
    grading: null,
    result: {
      isCorrect: true,
      score: 1,
      correct: q.options.filter((o) => o.correct).map((o) => o.id),
      accepted: q.accepted,
      explanation: q.explanation,
      rubric: q.rubric ?? [],
      tolerance: q.tolerance ?? null,
      matchMode: q.matchMode ?? 'auto',
    },
  };
}

/**
 * Giáo án (M34): mẫu = khung khối ban đầu. Mọi ô chữ là Markdown + công thức `$…$` (soạn bằng MarkdownEditor).
 *
 * Loại khối (khớp `backend/crates/core/src/modules/lesson_plans`):
 * - section       mục chữ ("I. Mục tiêu"…)
 * - activity      hoạt động theo CV 5512: a) Mục tiêu b) Nội dung c) Sản phẩm d) Tổ chức thực hiện (4 bước)
 * - two_columns   bảng "Hoạt động của GV | Hoạt động của HS" (CV 2345)
 * - problem_type  một dạng bài: phương pháp, ví dụ mẫu, bài tập (giáo án dạy thêm)
 * - questions     phiếu học tập / bài tập lấy từ ngân hàng câu hỏi
 */

let seq = 0;
/** Mã khối (giữ vị trí khi sắp xếp lại, làm key cho Vue). */
export const blockId = () => `b${Date.now().toString(36)}${(++seq).toString(36)}`;

export const BLOCK_KINDS = {
  section: 'Mục nội dung',
  activity: 'Hoạt động (CV 5512)',
  two_columns: 'Bảng GV / HS (CV 2345)',
  problem_type: 'Dạng bài (dạy thêm)',
  questions: 'Phiếu học tập / câu hỏi',
};

/** 4 bước tổ chức thực hiện một hoạt động (CV 5512). */
export const ACTIVITY_STEPS = [
  ['assign', 'Bước 1. Chuyển giao nhiệm vụ'],
  ['perform', 'Bước 2. Thực hiện nhiệm vụ'],
  ['report', 'Bước 3. Báo cáo, thảo luận'],
  ['conclude', 'Bước 4. Kết luận, nhận định'],
];

/** Các phần a) b) c) của hoạt động (d) là 4 bước ở trên). */
export const ACTIVITY_PARTS = [
  ['goal', 'a) Mục tiêu'],
  ['content', 'b) Nội dung'],
  ['product', 'c) Sản phẩm'],
];

export const PROBLEM_PARTS = [
  ['method', 'Phương pháp giải'],
  ['examples', 'Ví dụ mẫu (có lời giải)'],
  ['exercises', 'Bài tập vận dụng'],
];

export const INFO_FIELDS = [
  ['school', 'Trường'],
  ['group', 'Tổ chuyên môn'],
  ['teacherName', 'Họ và tên giáo viên'],
  ['periods', 'Thời lượng (số tiết)'],
  ['preparedOn', 'Ngày soạn'],
  ['taughtOn', 'Ngày dạy'],
  ['className', 'Lớp dạy'],
];

/** Khối trống theo loại. */
export function newBlock(kind, title = '') {
  const id = blockId();
  switch (kind) {
    case 'activity':
      return { id, kind, title, minutes: null, goal: '', content: '', product: '', steps: { assign: '', perform: '', report: '', conclude: '' } };
    case 'two_columns':
      return { id, kind, title, rows: [{ teacher: '', student: '' }] };
    case 'problem_type':
      return { id, kind, title, method: '', examples: '', exercises: '' };
    case 'questions':
      return { id, kind, title, questionIds: [], showAnswers: true };
    default:
      return { id, kind: 'section', title, body: '' };
  }
}

const section = (title, body = '') => ({ ...newBlock('section', title), body });
const activity = (title, minutes = null) => ({ ...newBlock('activity', title), minutes });

export const TEMPLATES = {
  cv5512: {
    label: 'CV 5512 (THCS, THPT)',
    description: 'Phụ lục IV Công văn 5512: Mục tiêu – Thiết bị – 4 hoạt động (a, b, c, d + 4 bước tổ chức).',
    blocks: () => [
      section('I. MỤC TIÊU', '**1. Kiến thức:**\n\n**2. Năng lực:**\n- Năng lực chung:\n- Năng lực đặc thù:\n\n**3. Phẩm chất:**'),
      section('II. THIẾT BỊ DẠY HỌC VÀ HỌC LIỆU', '**1. Giáo viên:**\n\n**2. Học sinh:**'),
      activity('Hoạt động 1: Mở đầu', 5),
      activity('Hoạt động 2: Hình thành kiến thức mới', 20),
      activity('Hoạt động 3: Luyện tập', 15),
      activity('Hoạt động 4: Vận dụng', 5),
    ],
  },
  cv2345: {
    label: 'CV 2345 (tiểu học)',
    description: 'Yêu cầu cần đạt – Đồ dùng dạy học – Các hoạt động dạy học chủ yếu (bảng GV / HS).',
    blocks: () => [
      section('I. YÊU CẦU CẦN ĐẠT'),
      section('II. ĐỒ DÙNG DẠY HỌC', '**1. Giáo viên:**\n\n**2. Học sinh:**'),
      {
        ...newBlock('two_columns', 'III. CÁC HOẠT ĐỘNG DẠY HỌC CHỦ YẾU'),
        rows: ['1. Khởi động', '2. Khám phá', '3. Luyện tập, thực hành', '4. Vận dụng, trải nghiệm'].map((t) => ({ teacher: `**${t}**`, student: '' })),
      },
      section('IV. ĐIỀU CHỈNH SAU BÀI DẠY'),
    ],
  },
  day_them: {
    label: 'Giáo án dạy thêm',
    description: 'Chuyên đề ôn luyện: mục tiêu – kiến thức cần nhớ – các dạng bài (phương pháp, ví dụ mẫu, bài tập) – bài tập về nhà.',
    blocks: () => [
      section('I. MỤC TIÊU'),
      section('II. KIẾN THỨC CẦN NHỚ'),
      newBlock('problem_type', 'Dạng 1:'),
      newBlock('problem_type', 'Dạng 2:'),
      newBlock('questions', 'III. BÀI TẬP TRẮC NGHIỆM'),
      section('IV. BÀI TẬP VỀ NHÀ'),
    ],
  },
  blank: {
    label: 'Tự do',
    description: 'Bắt đầu trống, tự thêm từng mục.',
    blocks: () => [section('')],
  },
};

export const TEMPLATE_OPTIONS = Object.entries(TEMPLATES).map(([value, t]) => ({ value, label: t.label }));

/** Tổng số phút của các hoạt động có ghi thời lượng. */
export function totalMinutes(blocks) {
  return (blocks ?? []).reduce((sum, b) => sum + (b.kind === 'activity' && Number.isFinite(b.minutes) ? b.minutes : 0), 0);
}

/** Đổi vị trí khối (dir = -1 lên, 1 xuống); trả mảng mới. */
export function moveBlock(blocks, index, dir) {
  const to = index + dir;
  if (to < 0 || to >= blocks.length) return blocks;
  const next = [...blocks];
  [next[index], next[to]] = [next[to], next[index]];
  return next;
}

/** Dữ liệu gửi api từ form của trang soạn. */
export function planPayload(form) {
  return {
    subjectId: form.subjectId,
    gradeId: form.gradeId || null,
    title: form.title.trim(),
    template: form.template,
    info: { ...form.info },
    blocks: form.blocks.map((b) => ({ ...b, minutes: b.kind === 'activity' && b.minutes !== '' ? (b.minutes ?? null) : undefined })),
    lessonId: form.lessonId || null,
    attachmentIds: (form.attachments ?? []).map((a) => a.id),
  };
}

/** Tên file Word: "Giao-an-Bai-3-Phuong-trinh-bac-hai.docx" (bỏ dấu, bỏ ký tự lạ). */
export function docxFileName(title) {
  const slug = String(title || 'giao-an')
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .replace(/đ/g, 'd')
    .replace(/Đ/g, 'D')
    .replace(/[^a-zA-Z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 80);
  return `Giao-an-${slug || 'moi'}.docx`;
}

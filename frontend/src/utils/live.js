/**
 * Phòng học trực tuyến (đợt 6): giao thức tin nhắn qua kênh dữ liệu của LiveKit + các hàm thuần (có test).
 *
 * Mọi tin là JSON `{ t: loại, … }` gửi kiểu "reliable". Vai trò người gửi lấy từ `metadata` do api ký trong
 * token (`{"role":"teacher"}`), người tham gia không tự sửa được → tin "của giáo viên" (cấp quyền vẽ, mở bình
 * chọn…) chỉ được chấp nhận khi người gửi đúng là giáo viên.
 *
 * | t | Ai gửi | Nội dung |
 * |---|---|---|
 * | hand | học sinh | `{ up, reason }` — giơ / hạ tay (reason: solve = "Em giải", ask = "Em hỏi") |
 * | hands-clear | giáo viên | hạ tay tất cả |
 * | chat | ai cũng gửi (trừ khách) | `{ text }` |
 * | chat-lock | giáo viên | `{ locked }` |
 * | stage | giáo viên | `{ mode: board \| screen \| camera }` — khung chính hiện gì |
 * | board-op | giáo viên + người được mời lên bảng | `{ op }` (thao tác của hui-whiteboard) |
 * | board-grant | giáo viên | `{ writers: [identity] }` — ai đang được vẽ |
 * | board-page | giáo viên | `{ page }` |
 * | board-sync-req | người vào sau | xin ảnh chụp bảng (giáo viên trả bằng text stream `board-snapshot`) |
 * | poll | giáo viên | `{ poll: { id, kind, question, options } }` |
 * | vote | học sinh → giáo viên | `{ id, choice }` |
 * | poll-results | giáo viên | `{ id, counts, voters, correct, closed }` |
 * | invite | giáo viên → một học sinh | `{ board, mic }` — mời lên bảng / bật mic |
 */

export const BOARD_SNAPSHOT_TOPIC = 'board-snapshot';
/** Bài trình chiếu giáo viên gửi cho cả lớp (nội dung dài nên đi text stream, không đi gói dữ liệu). */
export const SLIDES_TOPIC = 'slides-deck';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

export const encodeMessage = (msg) => encoder.encode(JSON.stringify(msg));

/** Tin hợp lệ (object có `t`) hoặc null. */
export function decodeMessage(payload) {
  try {
    const msg = JSON.parse(decoder.decode(payload));
    return msg && typeof msg === 'object' && typeof msg.t === 'string' ? msg : null;
  } catch {
    return null;
  }
}

/** Vai trò trong phòng từ metadata do server ký: teacher | assistant (trợ giảng của lớp) | student | guest. */
export function roleOf(participant) {
  try {
    return JSON.parse(participant?.metadata || '{}').role || 'student';
  } catch {
    return 'student';
  }
}

/** Người điều khiển phòng: giáo viên chủ buổi hoặc trợ giảng (tin các lệnh họ gửi qua kênh dữ liệu). */
export const isTeacher = (participant) => ['teacher', 'assistant'].includes(roleOf(participant));

export const HAND_REASONS = { solve: 'Em giải', ask: 'Em hỏi' };

/** Cập nhật hàng đợi giơ tay theo thứ tự giơ (giơ lại không đổi chỗ). */
export function updateHands(queue, identity, name, up, reason = 'solve', at = Date.now()) {
  const rest = queue.filter((h) => h.identity !== identity);
  if (!up) return rest;
  const existing = queue.find((h) => h.identity === identity);
  if (existing) return queue.map((h) => (h.identity === identity ? { ...h, reason } : h));
  return [...rest, { identity, name, reason, at }];
}

/** Phiếu bầu → số phiếu từng phương án (mỗi người một phiếu, bầu lại thì lấy phiếu sau). */
export function tally(votes, optionCount) {
  const counts = Array.from({ length: optionCount }, () => 0);
  for (const choice of votes.values()) {
    if (Number.isInteger(choice) && choice >= 0 && choice < optionCount) counts[choice] += 1;
  }
  return counts;
}

/** % làm tròn (0 khi chưa có phiếu). */
export const percent = (count, total) => (total ? Math.round((count / total) * 100) : 0);

/** Thời lượng đã học: 3725 giây → "1:02:05", 125 → "02:05". */
export function formatElapsed(seconds) {
  const s = Math.max(0, Math.floor(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const pad = (n) => String(n).padStart(2, '0');
  return h ? `${h}:${pad(m)}:${pad(s % 60)}` : `${pad(m)}:${pad(s % 60)}`;
}

/** Tên ngắn trên ô video: "Nguyễn Văn An" → "An", giữ nhãn "(Khách)". */
export function shortName(name) {
  const guest = /\(Khách\)$/.test(name || '');
  const base = String(name || '').replace(/\s*\(Khách\)$/, '').trim();
  const last = base.split(/\s+/).pop() || '?';
  return guest ? `${last} (Khách)` : last;
}

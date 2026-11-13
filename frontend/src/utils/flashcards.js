/** Thẻ ghi nhớ: định dạng khoảng ôn, hàng đợi ôn trong một lượt học. */

export const RATING_LABELS = { again: 'Quên', hard: 'Khó', good: 'Được', easy: 'Dễ' };

/** 10 → "10 phút"; 1440 → "1 ngày"; 43200 → "1 tháng". */
export function formatInterval(minutes) {
  const m = Math.max(0, Math.round(Number(minutes) || 0));
  if (m < 60) return `${m} phút`;
  if (m < 1440) return `${Math.round(m / 60)} giờ`;
  const d = Math.round(m / 1440);
  if (d < 30) return `${d} ngày`;
  if (d < 365) return `${Math.round(d / 30)} tháng`;
  return `${Math.round(d / 365)} năm`;
}

/**
 * Thẻ cần ôn trong lượt này: thẻ đến hạn (sớm nhất trước) rồi thẻ mới (theo thứ tự trong bộ).
 * @param {Array<{ id, state, dueAt }>} cards
 */
export function buildQueue(cards, now = Date.now()) {
  const t = new Date(now).getTime();
  const due = cards
    .filter((c) => c.state !== 'new' && c.dueAt && new Date(c.dueAt).getTime() <= t)
    .sort((a, b) => new Date(a.dueAt) - new Date(b.dueAt));
  const fresh = cards.filter((c) => c.state === 'new');
  return [...due, ...fresh];
}

/**
 * Sau khi chấm: bỏ thẻ khỏi hàng đợi; "Quên" thì đưa xuống cuối để học lại trong lượt.
 * @returns {Array} hàng đợi mới
 */
export function afterRating(queue, rating) {
  const [current, ...rest] = queue;
  if (!current) return [];
  return rating === 'again' ? [...rest, current] : rest;
}

import { describe, expect, it } from 'vitest';

import { afterRating, buildQueue, formatInterval } from './flashcards';

describe('flashcards utils', () => {
  it('formatInterval', () => {
    expect(formatInterval(10)).toBe('10 phút');
    expect(formatInterval(180)).toBe('3 giờ');
    expect(formatInterval(1440)).toBe('1 ngày');
    expect(formatInterval(5760)).toBe('4 ngày');
    expect(formatInterval(1440 * 60)).toBe('2 tháng');
    expect(formatInterval(1440 * 400)).toBe('1 năm');
  });

  it('buildQueue: thẻ đến hạn trước (sớm nhất trước), rồi thẻ mới; bỏ thẻ chưa tới hạn', () => {
    const now = '2026-11-13T12:00:00Z';
    const cards = [
      { id: 'n1', state: 'new', dueAt: null },
      { id: 'later', state: 'review', dueAt: '2026-11-14T00:00:00Z' },
      { id: 'd2', state: 'review', dueAt: '2026-11-13T11:00:00Z' },
      { id: 'd1', state: 'learning', dueAt: '2026-11-13T10:00:00Z' },
      { id: 'n2', state: 'new', dueAt: null },
    ];
    expect(buildQueue(cards, now).map((c) => c.id)).toEqual(['d1', 'd2', 'n1', 'n2']);
  });

  it('afterRating: Quên thì học lại cuối lượt', () => {
    const q = [{ id: 'a' }, { id: 'b' }, { id: 'c' }];
    expect(afterRating(q, 'again').map((c) => c.id)).toEqual(['b', 'c', 'a']);
    expect(afterRating(q, 'good').map((c) => c.id)).toEqual(['b', 'c']);
    expect(afterRating([], 'good')).toEqual([]);
  });
});

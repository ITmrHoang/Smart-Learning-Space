import { describe, expect, it } from 'vitest';

import {
  addDaysISO,
  formatDuration,
  formatSessionTime,
  fromLocal,
  isoWeekday,
  localParts,
  mergeSessions,
  sessionPhase,
  timeUntil,
  toTimetableEvents,
} from './schedule';

describe('schedule utils', () => {
  it('formatDuration', () => {
    expect(formatDuration(45)).toBe('45 phút');
    expect(formatDuration(90)).toBe('1 giờ 30 phút');
    expect(formatDuration(120)).toBe('2 giờ');
  });

  it('fromLocal ↔ localParts giữ nguyên ngày giờ của máy', () => {
    const iso = fromLocal('2026-11-10', '19:30');
    expect(iso).toMatch(/Z$/);
    expect(localParts(iso)).toEqual({ date: '2026-11-10', time: '19:30' });
    expect(fromLocal('', '19:30')).toBeNull();
    expect(fromLocal('2026-13-40', 'xx')).toBeNull();
  });

  it('isoWeekday / addDaysISO', () => {
    expect(isoWeekday('2026-11-09')).toBe(1); // thứ Hai
    expect(isoWeekday('2026-11-15')).toBe(7); // Chủ nhật
    expect(addDaysISO('2026-11-30', 1)).toBe('2026-12-01');
    expect(addDaysISO('2026-01-01', -1)).toBe('2025-12-31');
  });

  it('formatSessionTime', () => {
    const s = new Date(2026, 10, 10, 19, 30);
    const e = new Date(2026, 10, 10, 21, 0);
    expect(formatSessionTime(s, e)).toBe('19:30 – 21:00, thứ Ba 10/11/2026');
    expect(formatSessionTime(new Date(2026, 10, 10, 23, 0), new Date(2026, 10, 11, 0, 30))).toBe('23:00 – 00:30 11/11, thứ Ba 10/11/2026');
    expect(formatSessionTime(s, s)).toBe('19:30, thứ Ba 10/11/2026');
  });

  it('sessionPhase theo thời điểm', () => {
    const s = { status: 'scheduled', startsAt: '2026-11-10T12:30:00Z', endsAt: '2026-11-10T14:00:00Z' };
    expect(sessionPhase(s, '2026-11-10T12:00:00Z')).toBe('upcoming');
    expect(sessionPhase(s, '2026-11-10T12:21:00Z')).toBe('soon');
    expect(sessionPhase(s, '2026-11-10T13:00:00Z')).toBe('live');
    expect(sessionPhase(s, '2026-11-10T14:01:00Z')).toBe('ended');
    expect(sessionPhase({ ...s, status: 'cancelled' }, '2026-11-10T13:00:00Z')).toBe('cancelled');
  });

  it('timeUntil', () => {
    const now = '2026-11-10T12:00:00Z';
    expect(timeUntil('2026-11-10T12:25:00Z', now)).toBe('còn 25 phút');
    expect(timeUntil('2026-11-10T15:10:00Z', now)).toBe('còn 3 giờ');
    expect(timeUntil('2026-11-13T12:00:00Z', now)).toBe('còn 3 ngày');
    expect(timeUntil('2026-11-10T11:00:00Z', now)).toBe('');
  });

  it('toTimetableEvents: màu theo vai trò / trả lời, phụ đề', () => {
    const base = { startsAt: 'a', endsAt: 'b', status: 'scheduled', classTitle: 'Toán 9', teacherName: 'Cô Lan' };
    const [own, invited, declined] = toTimetableEvents([
      { ...base, id: 1, title: 'A', isOwner: true },
      { ...base, id: 2, title: 'B', isOwner: false, myRsvp: null },
      { ...base, id: 3, title: 'C', isOwner: false, myRsvp: 'no', classTitle: null },
    ]);
    expect(own).toMatchObject({ id: 1, color: 'primary', subtitle: 'Toán 9', start: 'a', end: 'b' });
    expect(invited).toMatchObject({ color: 'success', subtitle: 'Toán 9 · Cô Lan' });
    expect(declined).toMatchObject({ color: 'secondary', subtitle: 'Cô Lan' });
  });

  it('mergeSessions bỏ trùng và sắp theo giờ', () => {
    const a = { id: 'a', startsAt: '2026-11-10T12:00:00Z' };
    const b = { id: 'b', startsAt: '2026-11-09T12:00:00Z' };
    expect(mergeSessions([a], [b, a]).map((s) => s.id)).toEqual(['b', 'a']);
  });
});

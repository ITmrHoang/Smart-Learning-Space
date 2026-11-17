import { describe, expect, it } from 'vitest';

import { decodeMessage, encodeMessage, formatElapsed, isTeacher, percent, roleOf, shortName, tally, updateHands } from './live';

describe('phòng học trực tuyến', () => {
  it('mã hóa / giải mã tin; bỏ tin hỏng', () => {
    expect(decodeMessage(encodeMessage({ t: 'chat', text: 'Chào cô ạ' }))).toEqual({ t: 'chat', text: 'Chào cô ạ' });
    expect(decodeMessage(new TextEncoder().encode('{"x":1}'))).toBeNull();
    expect(decodeMessage(new TextEncoder().encode('không phải JSON'))).toBeNull();
  });

  it('vai trò lấy từ metadata do server ký', () => {
    expect(roleOf({ metadata: '{"role":"teacher"}' })).toBe('teacher');
    expect(roleOf({ metadata: '{"role":"guest"}' })).toBe('guest');
    expect(roleOf({ metadata: '' })).toBe('student');
    expect(roleOf({ metadata: 'hỏng' })).toBe('student');
    expect(isTeacher({ metadata: '{"role":"assistant"}' })).toBe(true);
    expect(isTeacher({ metadata: '{"role":"teacher"}' })).toBe(true);
    expect(isTeacher({ metadata: '{"role":"guest"}' })).toBe(false);
  });

  it('hàng đợi giơ tay giữ thứ tự, giơ lại không chen lên', () => {
    let q = [];
    q = updateHands(q, 'u:1', 'An', true, 'solve', 1);
    q = updateHands(q, 'u:2', 'Bình', true, 'ask', 2);
    q = updateHands(q, 'u:1', 'An', true, 'ask', 3);
    expect(q.map((h) => [h.identity, h.reason, h.at])).toEqual([
      ['u:1', 'ask', 1],
      ['u:2', 'ask', 2],
    ]);
    q = updateHands(q, 'u:1', 'An', false);
    expect(q.map((h) => h.identity)).toEqual(['u:2']);
  });

  it('đếm phiếu: mỗi người một phiếu, bỏ phiếu sai', () => {
    const votes = new Map([
      ['u:1', 1],
      ['u:2', 1],
      ['u:3', 0],
      ['u:4', 7],
    ]);
    votes.set('u:3', 2);
    expect(tally(votes, 3)).toEqual([0, 2, 1]);
    expect(percent(1, 3)).toBe(33);
    expect(percent(0, 0)).toBe(0);
  });

  it('định dạng thời gian và tên ngắn', () => {
    expect(formatElapsed(125)).toBe('02:05');
    expect(formatElapsed(3725)).toBe('1:02:05');
    expect(shortName('Nguyễn Văn An')).toBe('An');
    expect(shortName('Phụ huynh Lan (Khách)')).toBe('Lan (Khách)');
  });
});

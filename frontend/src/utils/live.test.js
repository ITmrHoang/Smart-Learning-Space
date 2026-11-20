import { describe, expect, it } from 'vitest';

import { choiceCounts, decodeMessage, encodeMessage, formatElapsed, gradeChoice, isTeacher, percent, publicQuestion, roleOf, shortName, tally, updateHands } from './live';

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

describe('câu hỏi từ ngân hàng', () => {
  const single = { id: 'q1', kind: 'single', content: '1 + 1?', matchMode: 'auto', options: [{ id: 'a', content: '1', correct: false }, { id: 'b', content: '2', correct: true }] };
  const multi = { id: 'q2', kind: 'multiple', options: [{ id: 'a', correct: true }, { id: 'b', correct: false }, { id: 'c', correct: true }] };
  const tf = { id: 'q3', kind: 'true_false', options: [{ id: 'a', correct: true }, { id: 'b', correct: false }] };

  it('gửi cho học sinh không lộ đáp án', () => {
    const p = publicQuestion(single, 'x1');
    expect(p).toEqual({ id: 'x1', questionId: 'q1', qkind: 'single', content: '1 + 1?', options: [{ id: 'a', content: '1' }, { id: 'b', content: '2' }], expression: false });
    expect(JSON.stringify(p)).not.toContain('correct');
    expect(publicQuestion({ ...single, kind: 'short', options: [], matchMode: 'expression' }, 'x2').expression).toBe(true);
  });

  it('chấm trắc nghiệm / nhiều đáp án / đúng sai và đếm lượt chọn', () => {
    expect(gradeChoice(single, { selected: ['b'] })).toBe(true);
    expect(gradeChoice(single, { selected: ['a'] })).toBe(false);
    expect(gradeChoice(single, null)).toBe(false);
    expect(gradeChoice(multi, { selected: ['c', 'a'] })).toBe(true);
    expect(gradeChoice(multi, { selected: ['a'] })).toBe(false);
    expect(gradeChoice(tf, { truth: { a: true, b: false } })).toBe(true);
    expect(gradeChoice(tf, { truth: { a: true } })).toBe(false);
    expect(choiceCounts(multi, [{ selected: ['a'] }, { selected: ['a', 'c'] }, null])).toEqual([2, 0, 1]);
    expect(choiceCounts(tf, [{ truth: { a: true, b: true } }, { truth: { a: false } }])).toEqual([1, 1]);
  });
});

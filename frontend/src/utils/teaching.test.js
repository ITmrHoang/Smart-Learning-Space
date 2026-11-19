import { describe, expect, it } from 'vitest';

import { formatClock, formatScore, isAnswered, letter, questionPoints, rubricTotal, secondsLeft, videoEmbed } from './teaching';

describe('video bài giảng', () => {
  it('nhận các dạng link YouTube → nhúng youtube-nocookie', () => {
    for (const url of [
      'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
      'https://youtu.be/dQw4w9WgXcQ',
      'https://m.youtube.com/watch?v=dQw4w9WgXcQ&list=x',
      'https://www.youtube.com/shorts/dQw4w9WgXcQ',
      'https://www.youtube.com/embed/dQw4w9WgXcQ',
    ]) {
      expect(videoEmbed(url)).toEqual({ type: 'youtube', src: 'https://www.youtube-nocookie.com/embed/dQw4w9WgXcQ' });
    }
    expect(videoEmbed('https://youtu.be/dQw4w9WgXcQ?t=90').src).toContain('?start=90');
  });

  it('link file → thẻ video; link xấu → null', () => {
    expect(videoEmbed('https://cdn.example.com/bai1.mp4')).toEqual({ type: 'file', src: 'https://cdn.example.com/bai1.mp4' });
    expect(videoEmbed('javascript:alert(1)')).toBeNull();
    expect(videoEmbed('không phải link')).toBeNull();
    expect(videoEmbed('')).toBeNull();
    expect(videoEmbed('https://youtube.com/watch?v=<script>')).toBeNull();
  });
});

describe('làm bài', () => {
  it('đồng hồ theo giờ server', () => {
    // Máy học sinh chạy nhanh 60 giây so với server.
    const loadedAt = Date.parse('2026-11-06T08:01:00Z');
    const left = secondsLeft('2026-11-06T08:10:00Z', '2026-11-06T08:00:00Z', loadedAt, loadedAt + 30_000);
    expect(left).toBe(570);
    expect(secondsLeft(null, '', 0)).toBeNull();
    expect(secondsLeft('2026-11-06T08:00:00Z', '2026-11-06T09:00:00Z', 0, 0)).toBe(0);
    expect(formatClock(125)).toBe('02:05');
    expect(formatClock(3725)).toBe('1:02:05');
  });

  it('câu đã làm', () => {
    const opts = [{ id: 'a' }, { id: 'b' }];
    expect(isAnswered({ kind: 'single', options: opts, answer: { selected: ['a'] } })).toBe(true);
    expect(isAnswered({ kind: 'single', options: opts, answer: null })).toBe(false);
    expect(isAnswered({ kind: 'true_false', options: opts, answer: { truth: { a: true } } })).toBe(false);
    expect(isAnswered({ kind: 'true_false', options: opts, answer: { truth: { a: true, b: false } } })).toBe(true);
    expect(isAnswered({ kind: 'short', options: [], answer: { text: '  ' } })).toBe(false);
  });

  it('định dạng điểm và chữ cái phương án', () => {
    expect(formatScore(7.25)).toBe('7,25');
    expect(formatScore(null)).toBe('—');
    expect(letter(2)).toBe('C');
  });
});

describe('câu tự luận', () => {
  const essay = { id: 'e1', kind: 'essay', rubric: [{ points: 0.25 }, { points: 0.5 }, { points: 0.25 }] };
  const mc = { id: 'q1', kind: 'single', rubric: [] };

  it('tổng điểm các bước chấm', () => {
    expect(rubricTotal(essay.rubric)).toBe(1);
    expect(rubricTotal([{ points: 0.1 }, { points: 0.2 }])).toBe(0.3);
    expect(rubricTotal(undefined)).toBe(0);
  });

  it('điểm câu cố định: điểm riêng → tổng bước (tự luận) → điểm của phần', () => {
    const section = { pointsPerQuestion: 2, questionPoints: {} };
    expect(questionPoints(section, mc)).toBe(2);
    expect(questionPoints(section, essay)).toBe(1);
    expect(questionPoints(section, { ...essay, rubric: [] })).toBe(2);
    expect(questionPoints({ ...section, questionPoints: { e1: 3 } }, essay)).toBe(3);
    expect(questionPoints({ ...section, questionPoints: { q1: 0 } }, mc)).toBe(0);
  });

  it('đã làm khi gõ bài hoặc có ảnh', () => {
    const it = (answer) => ({ kind: 'essay', options: [], answer });
    expect(isAnswered(it(null))).toBe(false);
    expect(isAnswered(it({ text: '  ' }))).toBe(false);
    expect(isAnswered(it({ text: 'x = 2' }))).toBe(true);
    expect(isAnswered(it({ files: ['f1'] }))).toBe(true);
  });
});

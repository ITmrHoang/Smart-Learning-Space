import { describe, expect, it } from 'vitest';

import { formatClock, formatScore, isAnswered, letter, secondsLeft, videoEmbed } from './teaching';

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

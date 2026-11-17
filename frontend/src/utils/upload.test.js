import { describe, expect, it } from 'vitest';

import { DOCX, altText, checkFile, fileType, formatBytes, imageMarkdown } from './upload';
import { formatWindows, groupSlotsByDay } from './schedule';

describe('upload', () => {
  it('kiểm tra loại + kích thước theo mục đích', () => {
    expect(checkFile({ type: 'image/png', size: 1000 }, 'content')).toBe('');
    expect(checkFile({ type: 'application/pdf', size: 10 }, 'content')).toMatch(/Chỉ nhận ảnh/);
    expect(checkFile({ type: 'application/pdf', size: 10 }, 'homework')).toBe('');
    expect(checkFile({ type: 'image/png', size: 6 * 1024 * 1024 }, 'content')).toBe('File tối đa 5 MB.');
    expect(checkFile({ type: 'image/png', size: 6 * 1024 * 1024 }, 'homework')).toBe('');
    expect(checkFile({ type: 'image/png', size: 0 }, 'homework')).toBe('File rỗng.');
    expect(checkFile({ type: 'image/svg+xml', size: 10 }, 'homework')).toMatch(/PDF/);
    // File Word: máy không cài Office báo type rỗng → đoán theo đuôi
    expect(checkFile({ type: DOCX, name: 'de.docx', size: 10 }, 'import')).toBe('');
    expect(checkFile({ type: '', name: 'Đề 15 phút.DOCX', size: 10 }, 'import')).toBe('');
    expect(fileType({ type: '', name: 'de.docx' })).toBe(DOCX);
    expect(checkFile({ type: 'application/msword', name: 'de.doc', size: 10 }, 'import')).toMatch(/\.docx/);
    expect(checkFile({ type: DOCX, name: 'de.docx', size: 21 * 1024 * 1024 }, 'import')).toBe('File tối đa 20 MB.');
  });

  it('chữ thay thế + Markdown ảnh, định dạng dung lượng', () => {
    expect(altText('Hình [1] (a).PNG')).toBe('Hình 1 a');
    expect(imageMarkdown({ name: 'tam-giac.png', url: '/api/files/abc' })).toBe('![tam-giac](/api/files/abc)');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1536 * 1024)).toBe('1,5 MB');
  });
});

describe('học kèm', () => {
  it('nhóm khoảng trống theo ngày, mô tả khung giờ', () => {
    const at = (d, h) => new Date(2026, 10, d, h, 0).toISOString();
    const days = groupSlotsByDay([
      { startsAt: at(16, 19), endsAt: at(16, 20) },
      { startsAt: at(16, 20), endsAt: at(16, 21) },
      { startsAt: at(18, 14), endsAt: at(18, 15) },
    ]);
    expect(days.map((d) => [d.label, d.slots.map((s) => s.time)])).toEqual([
      ['thứ Hai, 16/11/2026', ['19:00', '20:00']],
      ['thứ Tư, 18/11/2026', ['14:00']],
    ]);
    expect(
      formatWindows([
        { weekday: 3, start: '14:00', end: '16:00' },
        { weekday: 1, start: '19:00', end: '21:00' },
      ]),
    ).toBe('T2 19:00–21:00, T4 14:00–16:00');
  });
});

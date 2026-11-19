import { describe, expect, it } from 'vitest';

import { splitSlides } from './slides';

describe('splitSlides', () => {
  it('tách theo dòng --- , bỏ slide trống, không tách trong khối code', () => {
    const md = '# Bài 3\n\nMở đầu\n\n---\n\n## Định nghĩa\n\n```\na\n---\nb\n```\n\n---\n\n---\n\nLuyện tập';
    expect(splitSlides(md)).toEqual(['# Bài 3\n\nMở đầu', '## Định nghĩa\n\n```\na\n---\nb\n```', 'Luyện tập']);
  });

  it('bài không có --- thì mỗi tiêu đề # / ## mở slide mới', () => {
    expect(splitSlides('# Bài 3\nGiới thiệu\n## Phần 1\nA\n### Ý nhỏ\nB\n## Phần 2\nC')).toEqual([
      '# Bài 3\nGiới thiệu',
      '## Phần 1\nA\n### Ý nhỏ\nB',
      '## Phần 2\nC',
    ]);
    expect(splitSlides('Chỉ một đoạn')).toEqual(['Chỉ một đoạn']);
    expect(splitSlides('')).toEqual([]);
    expect(splitSlides(null)).toEqual([]);
  });
});

import { describe, expect, it } from 'vitest';

import { blocks } from './docx/fromDom';
import { answerCode, optionColumns, renderAllPapersDom, renderAnswerKeyDom, renderPaperDom, sectionsOf } from './examPaperRender';

const opt = (content, correct = false) => ({ id: content, content, correct });
const q = (number, kind, extra = {}) => ({
  number,
  sectionPosition: 0,
  sectionTitle: 'PHẦN I',
  points: 0.25,
  kind,
  content: `Câu hỏi ${number}: $x^2$`,
  options: [],
  accepted: [],
  explanation: '',
  rubric: [],
  ...extra,
});

const paper = (code, keyIndex) => ({
  code,
  items: [
    q(1, 'single', { options: ['1', '2', '3', '4'].map((c, i) => opt(c, i === keyIndex)) }),
    q(2, 'true_false', { options: [opt('a', true), opt('b'), opt('c', true), opt('d')] }),
    q(1, 'short', { sectionPosition: 1, sectionTitle: 'PHẦN III', accepted: ['0,5'] }),
    q(1, 'essay', {
      sectionPosition: 2,
      sectionTitle: 'PHẦN IV. Tự luận',
      points: 1.5,
      content: 'Chứng minh bất đẳng thức.',
      rubric: [{ id: 's1', content: 'Biến đổi tương đương', points: 1 }, { id: 's2', content: 'Kết luận', points: 0.5 }],
      explanation: 'Dùng AM-GM.',
    }),
  ],
});

describe('mã đề', () => {
  it('xếp phương án 4 / 2 / 1 cột theo độ dài', () => {
    expect(optionColumns(['1', '2', '3', '4'].map((c) => opt(c)))).toBe(4);
    expect(optionColumns([opt('Hàm số đồng biến trên R'), opt('Hàm số nghịch biến trên R')])).toBe(2);
    expect(optionColumns([opt('Một câu trả lời rất dài, dài hơn bốn mươi ký tự để phải xuống dòng riêng')])).toBe(1);
    expect(optionColumns([opt('$$x^2$$'), opt('2')])).toBe(1);
  });

  it('ký hiệu đáp án theo dạng câu', () => {
    const p = paper('101', 2);
    expect(p.items.map(answerCode)).toEqual(['C', 'ĐSĐS', '0,5', 'Tự luận']);
    expect(answerCode(q(1, 'multiple', { options: [opt('a', true), opt('b'), opt('c', true)] }))).toBe('AC');
  });

  it('gom câu theo phần', () => {
    expect(sectionsOf(paper('101', 0).items).map((s) => [s.title, s.items.length])).toEqual([
      ['PHẦN I', 2],
      ['PHẦN III', 1],
      ['PHẦN IV. Tự luận', 1],
    ]);
  });

  it('một mã đề: đầu đề, mã đề, câu, phương án, HẾT', () => {
    const el = renderPaperDom(paper('101', 0), { school: 'Trung tâm Ánh Sao', examName: 'Kiểm tra giữa kỳ', subjectName: 'Toán', minutes: 45 });
    const text = el.textContent;
    expect(text).toContain('TRUNG TÂM ÁNH SAO');
    expect(text).toContain('KIỂM TRA GIỮA KỲ');
    expect(text).toContain('Môn: Toán');
    expect(text).toContain('Thời gian làm bài: 45 phút');
    expect(text).toContain('Mã đề 101');
    expect(text).toContain('Câu 1.');
    expect(text).toContain('(1,5 điểm)');
    expect(text).toContain('------ HẾT ------');
    expect(el.querySelector('table.options').querySelectorAll('td')).toHaveLength(4);
    expect(el.querySelectorAll('p.statement')).toHaveLength(4);
  });

  it('nhiều mã đề sang trang mới; Word có ngắt trang, bảng phương án không viền', () => {
    const all = renderAllPapersDom([paper('101', 0), paper('102', 3)]);
    expect(all.querySelectorAll('.page-break')).toHaveLength(1);
    const xml = blocks(all, { media: new Map() }).join('');
    expect(xml).toContain('<w:br w:type="page"/>');
    expect(xml).toContain('w:val="none"');
  });

  it('bảng đáp án câu × mã đề + hướng dẫn chấm tự luận', () => {
    const el = renderAnswerKeyDom([paper('101', 0), paper('102', 3)], { examName: 'Kiểm tra' });
    const first = el.querySelector('table.key');
    expect([...first.querySelectorAll('th')].map((t) => t.textContent)).toEqual(['Câu', 'Mã 101', 'Mã 102']);
    expect([...first.querySelectorAll('tbody tr')[0].children].map((c) => c.textContent)).toEqual(['1', 'A', 'D']);
    expect(el.textContent).toContain('HƯỚNG DẪN CHẤM TỰ LUẬN');
    expect(el.querySelector('table.rubric').textContent).toContain('Kết luận');
    // Câu tự luận giống nhau ở 2 mã chỉ ghi một lần
    expect(el.querySelectorAll('table.rubric')).toHaveLength(1);
  });
});

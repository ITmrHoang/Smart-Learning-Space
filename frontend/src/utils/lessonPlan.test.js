import { describe, expect, it } from 'vitest';

import { TEMPLATES, docxFileName, moveBlock, newBlock, planPayload, totalMinutes } from './lessonPlan';

describe('lessonPlan', () => {
  it('mẫu CV 5512 có mục tiêu, thiết bị và đủ 4 hoạt động; mã khối không trùng', () => {
    const blocks = TEMPLATES.cv5512.blocks();
    expect(blocks.map((b) => b.kind)).toEqual(['section', 'section', 'activity', 'activity', 'activity', 'activity']);
    expect(blocks[2].title).toBe('Hoạt động 1: Mở đầu');
    expect(blocks[5].title).toBe('Hoạt động 4: Vận dụng');
    expect(new Set(blocks.map((b) => b.id)).size).toBe(blocks.length);
    expect(totalMinutes(blocks)).toBe(45);
    // Mỗi lần tạo là bản mới (sửa giáo án này không ảnh hưởng mẫu)
    expect(TEMPLATES.cv5512.blocks()[0]).not.toBe(blocks[0]);
  });

  it('mẫu CV 2345 có bảng GV / HS; mẫu dạy thêm có dạng bài + phiếu câu hỏi', () => {
    const tableBlock = TEMPLATES.cv2345.blocks().find((b) => b.kind === 'two_columns');
    expect(tableBlock.rows).toHaveLength(4);
    expect(TEMPLATES.day_them.blocks().map((b) => b.kind)).toContain('problem_type');
    expect(TEMPLATES.day_them.blocks().map((b) => b.kind)).toContain('questions');
  });

  it('khối mới, sắp xếp, dữ liệu gửi api, tên file Word', () => {
    expect(newBlock('activity').steps).toEqual({ assign: '', perform: '', report: '', conclude: '' });
    expect(newBlock('lạ').kind).toBe('section');
    const a = newBlock('section', 'A');
    const b = newBlock('section', 'B');
    expect(moveBlock([a, b], 1, -1).map((x) => x.title)).toEqual(['B', 'A']);
    expect(moveBlock([a, b], 0, -1)).toEqual([a, b]);
    const p = planPayload({
      subjectId: 's', gradeId: '', title: '  Bài 1  ', template: 'cv5512', info: { school: 'THCS' },
      blocks: [a, { ...newBlock('activity'), minutes: 10 }], lessonId: '', attachments: [{ id: 'f1' }],
    });
    expect(p.title).toBe('Bài 1');
    expect(p.gradeId).toBeNull();
    expect(p.blocks[1].minutes).toBe(10);
    expect(p.attachmentIds).toEqual(['f1']);
    expect(docxFileName('Bài 3. Phương trình bậc hai — Đại số')).toBe('Giao-an-Bai-3-Phuong-trinh-bac-hai-Dai-so.docx');
  });
});

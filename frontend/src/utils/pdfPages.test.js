import { describe, expect, it, vi } from 'vitest';
import { PDF_PAGE_HEIGHT, PdfFileError, importPdfPages, isPdf, pageName, pageScale } from './pdfPages';

const pdfFile = () => new File([new Uint8Array([0x25, 0x50, 0x44, 0x46])], 'Đề 15 phút.pdf', { type: 'application/pdf' });

/** Bộ đọc giả: `n` trang ảnh WebP. */
const fakeRender = (n, total = n) =>
  vi.fn(async (_file, { maxPages, onPage }) => {
    for (let i = 1; i <= Math.min(n, maxPages); i++) {
      await onPage({ index: i, total: Math.min(n, maxPages), blob: new Blob(['x'], { type: 'image/webp' }), width: 1273, height: 1800 });
    }
    return { total };
  });

describe('pdfPages', () => {
  it('tỉ lệ trang, tên ảnh, nhận ra PDF', () => {
    expect(842 * pageScale(595, 842)).toBeCloseTo(PDF_PAGE_HEIGHT);
    expect(5000 * pageScale(5000, 400)).toBeCloseTo(3200);
    expect(pageScale(0, 10)).toBe(1);
    expect(pageName('Đề 15 phút.pdf', 2)).toBe('Đề 15 phút - trang 2');
    expect(isPdf(pdfFile())).toBe(true);
    expect(isPdf(new File(['x'], 'de.PDF'))).toBe(true);
    expect(isPdf(new File(['x'], 'a.png', { type: 'image/png' }))).toBe(false);
  });

  it('trình duyệt đọc được → tải từng ảnh trang lên, không gọi máy chủ', async () => {
    const upload = vi.fn(async (f) => ({ url: `/api/files/${f.name}` }));
    const serverImport = vi.fn();
    const progress = [];
    const res = await importPdfPages(pdfFile(), {
      maxPages: 2,
      upload,
      serverImport,
      onProgress: (s) => progress.push(s),
      render: fakeRender(3),
    });
    expect(res.via).toBe('browser');
    expect(res.total).toBe(3);
    expect(res.pages).toEqual([
      { url: '/api/files/Đề 15 phút - trang 1.webp', width: 1273, height: 1800 },
      { url: '/api/files/Đề 15 phút - trang 2.webp', width: 1273, height: 1800 },
    ]);
    expect(upload.mock.calls[0][0].type).toBe('image/webp');
    expect(serverImport).not.toHaveBeenCalled();
    expect(progress.at(-1)).toEqual({ via: 'browser', done: 2, total: 2 });
  });

  it('trình duyệt không đọc được → máy chủ đọc (giữ giới hạn trang)', async () => {
    const serverImport = vi.fn(async () => ({ total: 5, pages: [{ url: '/api/files/1', width: 1273, height: 1800 }] }));
    const render = vi.fn(async () => {
      throw new Error('Không tải được pdf.worker');
    });
    vi.spyOn(console, 'warn').mockImplementation(() => {});
    const res = await importPdfPages(pdfFile(), { maxPages: 99, upload: vi.fn(), serverImport, render });
    expect(res).toEqual({ via: 'server', total: 5, pages: [{ url: '/api/files/1', width: 1273, height: 1800 }] });
    expect(serverImport.mock.calls[0][1]).toBe(30);
  });

  it('PDF có mật khẩu / lỗi tải ảnh lên → báo lỗi, không gửi máy chủ', async () => {
    const serverImport = vi.fn();
    const locked = vi.fn(async () => {
      throw new PdfFileError('PDF có mật khẩu');
    });
    await expect(importPdfPages(pdfFile(), { upload: vi.fn(), serverImport, render: locked })).rejects.toThrow('mật khẩu');
    const upload = vi.fn(async () => {
      throw new Error('Mất kết nối khi upload.');
    });
    await expect(importPdfPages(pdfFile(), { upload, serverImport, render: fakeRender(2) })).rejects.toThrow('Mất kết nối');
    expect(serverImport).not.toHaveBeenCalled();
  });
});

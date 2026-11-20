/**
 * Nhập đề PDF thành ảnh từng trang để đặt làm nền trang bảng — kiểu "hybrid":
 * 1. Trình duyệt tự đọc PDF bằng pdfjs-dist (chỉ tải thư viện khi giáo viên chọn PDF) → máy chủ không tốn gì.
 * 2. Trình duyệt không đọc được (máy / trình duyệt cũ, lỗi tải thư viện, lỗi vẽ) → gửi PDF lên máy chủ đọc
 *    (`POST /sessions/{id}/live/board/pdf`, Rust `hayro`).
 * PDF có mật khẩu / hỏng: báo luôn cho giáo viên, không gửi lên máy chủ (máy chủ cũng không đọc được).
 */

/** Tối đa số trang mỗi lần nhập (giống máy chủ). */
export const PDF_MAX_PAGES = 30;
/** Chiều cao ảnh mỗi trang: gấp đôi chiều cao trang bảng (900) để chữ vẫn rõ trên màn hình nét cao. */
export const PDF_PAGE_HEIGHT = 1800;
export const PDF_MAX_WIDTH = 3200;
/** Ảnh nội dung tối đa 5 MB. */
const MAX_IMAGE_BYTES = 5 * 1024 * 1024;

/** Lỗi do chính file PDF (mật khẩu, hỏng): báo giáo viên, không thử ở máy chủ. */
export class PdfFileError extends Error {}

/** Tỉ lệ phóng (1 = 1 pt / px) để ảnh trang cao `PDF_PAGE_HEIGHT`, không rộng quá `PDF_MAX_WIDTH`. */
export function pageScale(width, height) {
  if (!(width > 0 && height > 0)) return 1;
  return Math.min(PDF_PAGE_HEIGHT / height, PDF_MAX_WIDTH / width);
}

export const isPdf = (file) => file?.type === 'application/pdf' || /\.pdf$/i.test(file?.name ?? '');

/** "Đề 15 phút.pdf", 2 → "Đề 15 phút - trang 2" */
export const pageName = (fileName, index) => `${String(fileName || 'de').replace(/\.pdf$/i, '')} - trang ${index}`;

let pdfjsLoader = null;

/** Tải pdfjs-dist (một lần) — tách thành file riêng, không làm nặng trang lúc đầu. */
function loadPdfjs() {
  pdfjsLoader ??= Promise.all([import('pdfjs-dist'), import('pdfjs-dist/build/pdf.worker.min.mjs?url')])
    .then(([pdfjs, worker]) => {
      pdfjs.GlobalWorkerOptions.workerSrc = worker.default;
      return pdfjs;
    })
    .catch((e) => {
      pdfjsLoader = null;
      throw e;
    });
  return pdfjsLoader;
}

function toBlob(canvas, type, quality) {
  return new Promise((resolve) => canvas.toBlob(resolve, type, quality));
}

/** Ảnh trang: WebP (nhỏ, nét chữ rõ); trình duyệt không xuất được WebP → JPEG. Quá 5 MB thì giảm chất lượng. */
async function encodePage(canvas) {
  let blob = await toBlob(canvas, 'image/webp', 0.9);
  if (!blob || blob.type !== 'image/webp') blob = await toBlob(canvas, 'image/jpeg', 0.9);
  for (const q of [0.75, 0.6]) {
    if (blob && blob.size <= MAX_IMAGE_BYTES) break;
    blob = await toBlob(canvas, 'image/jpeg', q);
  }
  if (!blob) throw new Error('Trình duyệt không xuất được ảnh trang PDF.');
  return blob;
}

/**
 * Đọc PDF ngay trên trình duyệt: lần lượt từng trang → ảnh → `onPage({ index, total, blob, width, height })`
 * (đợi `onPage` xong mới vẽ trang sau, để bộ nhớ chỉ giữ một trang).
 * @returns {Promise<{ total: number }>} tổng số trang của file
 */
export async function renderPdfInBrowser(file, { maxPages = PDF_MAX_PAGES, onPage }) {
  const pdfjs = await loadPdfjs();
  const task = pdfjs.getDocument({
    data: new Uint8Array(await file.arrayBuffer()),
    isEvalSupported: false,
    // Font chuẩn, bảng mã chữ, bộ giải mã ảnh scan (JBIG2 / JPEG 2000): chỉ tải khi file cần
    standardFontDataUrl: '/pdfjs/standard_fonts/',
    cMapUrl: '/pdfjs/cmaps/',
    wasmUrl: '/pdfjs/wasm/',
    iccUrl: '/pdfjs/iccs/',
  });
  // Có mật khẩu: dừng luôn, nhờ giáo viên bỏ mật khẩu (không hỏi mật khẩu trong phòng học)
  task.onPassword = () => task.destroy();
  let doc;
  try {
    doc = await task.promise;
  } catch (e) {
    if (e?.name === 'PasswordException' || task.destroyed) {
      throw new PdfFileError('PDF có mật khẩu — hãy mở bằng trình đọc PDF, bỏ mật khẩu rồi thử lại.');
    }
    if (e?.name === 'InvalidPDFException') throw new PdfFileError('File PDF bị hỏng hoặc không phải PDF.');
    throw e;
  }
  try {
    const count = Math.min(doc.numPages, maxPages);
    for (let index = 1; index <= count; index++) {
      const page = await doc.getPage(index);
      const base = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: pageScale(base.width, base.height) });
      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      await page.render({ canvas, viewport, background: '#ffffff' }).promise;
      const blob = await encodePage(canvas);
      const size = { width: canvas.width, height: canvas.height };
      // Giải phóng bộ nhớ ảnh trước khi sang trang sau
      canvas.width = canvas.height = 0;
      page.cleanup();
      await onPage({ index, total: count, blob, ...size });
    }
    return { total: doc.numPages };
  } finally {
    // Giải phóng worker / bộ nhớ của file; lỗi lúc dọn dẹp không làm hỏng các trang đã đọc xong
    await task.destroy().catch(() => {});
  }
}

/** Lỗi khi tải ảnh lên (mạng, kho file) — không phải lỗi đọc PDF nên không chuyển sang máy chủ đọc. */
class UploadError extends Error {
  constructor(cause) {
    super(cause?.message ?? String(cause));
    this.cause = cause;
  }
}

/**
 * Nhập PDF thành ảnh từng trang (đã tải lên kho): thử ở trình duyệt trước, không được thì nhờ máy chủ.
 * @param {File} file
 * @param {object} opts
 * @param {number} [opts.maxPages]
 * @param {(file: File) => Promise<{ url: string }>} opts.upload - tải một ảnh trang lên (ảnh nội dung)
 * @param {(file: File, maxPages: number) => Promise<{ total: number, pages: Array<{ url, width, height }> }>} opts.serverImport
 * @param {(state: { via: 'browser'|'server', done?: number, total?: number }) => void} [opts.onProgress]
 * @param {Function} [opts.render] - thay bộ đọc ở trình duyệt (test)
 * @returns {Promise<{ via: 'browser'|'server', total: number, pages: Array<{ url, width, height }> }>}
 */
export async function importPdfPages(file, { maxPages = PDF_MAX_PAGES, upload, serverImport, onProgress, render = renderPdfInBrowser }) {
  const limit = Math.max(1, Math.min(PDF_MAX_PAGES, maxPages));
  const pages = [];
  try {
    onProgress?.({ via: 'browser' });
    const { total } = await render(file, {
      maxPages: limit,
      onPage: async ({ index, total: count, blob, width, height }) => {
        const ext = blob.type === 'image/webp' ? 'webp' : blob.type === 'image/png' ? 'png' : 'jpg';
        const image = new File([blob], `${pageName(file.name, index)}.${ext}`, { type: blob.type });
        let saved;
        try {
          saved = await upload(image);
        } catch (e) {
          throw new UploadError(e);
        }
        pages.push({ url: saved.url, width, height });
        onProgress?.({ via: 'browser', done: index, total: count });
      },
    });
    if (!pages.length) throw new PdfFileError('PDF không có trang nào.');
    return { via: 'browser', total, pages };
  } catch (e) {
    if (e instanceof PdfFileError) throw e;
    if (e instanceof UploadError) throw e.cause ?? e;
    // Trình duyệt không đọc được → máy chủ đọc lại cả file
    console.warn('Trình duyệt không đọc được PDF, chuyển sang máy chủ:', e);
    onProgress?.({ via: 'server' });
    const result = await serverImport(file, limit);
    return { via: 'server', total: result.total, pages: result.pages };
  }
}

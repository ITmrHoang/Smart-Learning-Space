/**
 * Upload file (M21): xin link → trình duyệt PUT thẳng lên Garage (qua `/files/*` cùng domain) → báo xong.
 * - `content`: ảnh trong nội dung (bài học, câu hỏi, hỏi đáp, hồ sơ) — tối đa 5 MB.
 * - `homework`: ảnh / PDF bài làm — tối đa 15 MB; ảnh chụp điện thoại quá lớn được thu nhỏ trước khi gửi.
 * - `import`: file tạm API đọc một lần rồi xóa — Word (.docx) đề câu hỏi, PDF đề đưa lên bảng (khi trình duyệt
 *   không tự đọc được) — tối đa 20 MB.
 * - `attachment`: tài liệu đính kèm bài học / giáo án (PDF, Word, Excel, PowerPoint) — tối đa 20 MB.
 */
import { fileApi } from '../api';

export const IMAGE_TYPES = ['image/png', 'image/jpeg', 'image/gif', 'image/webp'];
export const DOCX = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document';
export const XLSX = 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet';
export const PPTX = 'application/vnd.openxmlformats-officedocument.presentationml.presentation';
export const ATTACHMENT_TYPES = ['application/pdf', DOCX, XLSX, PPTX];
export const ATTACHMENT_ACCEPT = '.pdf,.docx,.xlsx,.pptx';
export const LIMITS = { content: 5 * 1024 * 1024, homework: 15 * 1024 * 1024, import: 20 * 1024 * 1024, attachment: 20 * 1024 * 1024 };

const BY_EXTENSION = { pdf: 'application/pdf', docx: DOCX, xlsx: XLSX, pptx: PPTX };

/** Loại file: máy không cài Office có thể báo rỗng cho .docx / .xlsx / .pptx → đoán theo đuôi. */
export function fileType(file) {
  if (file?.type) return file.type;
  const ext = /\.([a-z0-9]+)$/i.exec(file?.name ?? '')?.[1]?.toLowerCase();
  return BY_EXTENSION[ext] ?? '';
}

/** Lỗi trước khi upload ('' = hợp lệ). */
export function checkFile(file, purpose) {
  if (purpose === 'import') {
    if (![DOCX, 'application/pdf'].includes(fileType(file))) {
      return 'Chỉ nhận file Word .docx (file .doc cũ: mở bằng Word rồi lưu lại dạng .docx) hoặc PDF.';
    }
  } else if (purpose === 'attachment') {
    if (!ATTACHMENT_TYPES.includes(fileType(file))) return 'Chỉ nhận PDF, Word (.docx), Excel (.xlsx) hoặc PowerPoint (.pptx).';
  } else {
    const types = purpose === 'homework' ? [...IMAGE_TYPES, 'application/pdf'] : IMAGE_TYPES;
    if (!types.includes(file?.type)) {
      return purpose === 'homework' ? 'Chỉ nhận ảnh (PNG, JPG, GIF, WebP) hoặc PDF.' : 'Chỉ nhận ảnh PNG, JPG, GIF hoặc WebP.';
    }
  }
  if (!file.size) return 'File rỗng.';
  if (file.size > LIMITS[purpose]) return `File tối đa ${LIMITS[purpose] / 1024 / 1024} MB.`;
  return '';
}

/** 1572864 → "1,5 MB" */
export function formatBytes(n) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${Math.round(n / 1024)} KB`;
  return `${(n / 1024 / 1024).toLocaleString('vi-VN', { maximumFractionDigits: 1 })} MB`;
}

/** Tên file → chữ thay thế của ảnh trong Markdown (bỏ đuôi, bỏ ký tự làm hỏng cú pháp). */
export function altText(name) {
  return String(name || 'ảnh')
    .replace(/\.[a-z0-9]+$/i, '')
    .replace(/[[\]()]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

export const imageMarkdown = (f) => `![${altText(f.name)}](${f.url})`;

/** Ảnh JPEG / PNG cạnh dài hơn `max` px → JPEG thu nhỏ (giữ nguyên nếu không thu nhỏ được). */
export async function shrinkImage(file, max = 2200, quality = 0.85) {
  if (!['image/jpeg', 'image/png'].includes(file.type) || typeof createImageBitmap !== 'function') return file;
  try {
    const bitmap = await createImageBitmap(file);
    const scale = max / Math.max(bitmap.width, bitmap.height);
    if (scale >= 1) return file;
    const canvas = document.createElement('canvas');
    canvas.width = Math.round(bitmap.width * scale);
    canvas.height = Math.round(bitmap.height * scale);
    canvas.getContext('2d').drawImage(bitmap, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise((resolve) => canvas.toBlob(resolve, 'image/jpeg', quality));
    if (!blob || blob.size >= file.size) return file;
    return new File([blob], file.name.replace(/\.[a-z0-9]+$/i, '') + '.jpg', { type: 'image/jpeg' });
  } catch {
    return file;
  }
}

/** PUT file lên presigned URL (XHR để có tiến độ). */
function putFile(url, file, headers, onProgress) {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('PUT', url);
    Object.entries(headers ?? {}).forEach(([k, v]) => xhr.setRequestHeader(k, v));
    if (onProgress) {
      xhr.upload.onprogress = (e) => e.lengthComputable && onProgress(Math.round((e.loaded / e.total) * 100));
    }
    xhr.onload = () =>
      xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new Error(`Upload thất bại (HTTP ${xhr.status}).`));
    xhr.onerror = () => reject(new Error('Mất kết nối khi upload. Hãy thử lại.'));
    xhr.send(file);
  });
}

/** Upload một file → `{ id, name, contentType, size, isImage, url }`. */
export async function uploadFile(file, purpose, { onProgress, name } = {}) {
  const ready = purpose === 'homework' ? await shrinkImage(file) : file;
  const error = checkFile(ready, purpose);
  if (error) throw new Error(error);
  const ticket = await fileApi.ticket({
    purpose,
    name: name || ready.name || 'file',
    contentType: fileType(ready),
    size: ready.size,
  });
  await putFile(ticket.uploadUrl, ready, ticket.headers, onProgress);
  return fileApi.complete(ticket.id);
}

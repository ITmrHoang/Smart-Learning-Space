/**
 * Bài học → slide trình chiếu (M36).
 * - Dòng chỉ có `---` (ngoài khối code) là chỗ sang slide mới.
 * - Bài không có `---` nào: mỗi tiêu đề `#` / `##` mở một slide mới (bài soạn sẵn vẫn trình chiếu được).
 * Bỏ slide trống.
 */

const FENCE = /^\s*(```|~~~)/;
const SEPARATOR = /^\s*---\s*$/;
const HEADING = /^#{1,2}\s/;

export function splitSlides(markdown) {
  const lines = String(markdown ?? '').replace(/\r\n?/g, '\n').split('\n');
  const hasSeparator = (() => {
    let inFence = false;
    for (const line of lines) {
      if (FENCE.test(line)) inFence = !inFence;
      else if (!inFence && SEPARATOR.test(line)) return true;
    }
    return false;
  })();

  const slides = [];
  let current = [];
  let inFence = false;
  const push = () => {
    const text = current.join('\n').trim();
    if (text) slides.push(text);
    current = [];
  };
  for (const line of lines) {
    if (FENCE.test(line)) {
      inFence = !inFence;
    } else if (!inFence) {
      if (hasSeparator && SEPARATOR.test(line)) {
        push();
        continue;
      }
      if (!hasSeparator && HEADING.test(line) && current.some((l) => l.trim())) push();
    }
    current.push(line);
  }
  push();
  return slides;
}

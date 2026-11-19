/**
 * Mã đề → DOM thuần (không shadow DOM) cho trang in và xuất Word (`utils/docx`), trình bày kiểu đề THPT:
 * đầu đề (đơn vị / kỳ kiểm tra / môn / thời gian), "Họ, tên thí sinh … Số báo danh …", "Mã đề", các PHẦN,
 * phương án A B C D xếp 4 / 2 / 1 cột theo độ dài, "HẾT". Bảng đáp án: câu × mã đề + hướng dẫn chấm tự luận.
 */
import { renderMarkdown } from '@itmrhoang/h-ui/hui-markdown';

import { formatScore, letter, statementLetter } from './teaching';

function h(tag, attrs, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs ?? {})) {
    if (v != null && v !== false) el.setAttribute(k, v === true ? '' : String(v));
  }
  for (const c of children.flat()) {
    if (c == null || c === false) continue;
    el.append(typeof c === 'string' ? document.createTextNode(c) : c);
  }
  return el;
}

const md = (text) => (String(text ?? '').trim() ? renderMarkdown(String(text).trim()) : []);
const mdInline = (text) => (String(text ?? '').trim() ? renderMarkdown(String(text).trim(), { inline: true }) : []);

/** Độ dài "nhìn thấy" của phương án (công thức tính khoảng một nửa số ký tự LaTeX). */
function visibleLength(content) {
  return String(content)
    .replace(/\$([^$]*)\$/g, (_, tex) => 'x'.repeat(Math.ceil(tex.replace(/\\[a-z]+/gi, 'x').length / 2)))
    .replace(/[*_`]/g, '').length;
}

/** Số cột xếp phương án: đều ngắn → 4, vừa → 2, dài / có ảnh / nhiều dòng → 1. */
export function optionColumns(options) {
  if (!options.length) return 1;
  if (options.some((o) => /\n|!\[|```|\$\$|^\s*plot/.test(o.content))) return 1;
  const longest = Math.max(...options.map((o) => visibleLength(o.content)));
  if (longest <= 18 && options.length <= 4) return 4;
  if (longest <= 40) return 2;
  return 1;
}

/** Ký hiệu đáp án của một câu trong bảng đáp án. */
export function answerCode(q) {
  switch (q.kind) {
    case 'single':
    case 'multiple':
      return q.options.map((o, i) => (o.correct ? letter(i) : null)).filter(Boolean).join('');
    case 'true_false':
      return q.options.map((o) => (o.correct ? 'Đ' : 'S')).join('');
    case 'short':
      return q.accepted?.[0] ?? '';
    default:
      return 'Tự luận';
  }
}

/** Gom câu theo phần (giữ thứ tự). */
export function sectionsOf(items) {
  const out = [];
  for (const it of items) {
    let s = out[out.length - 1];
    if (!s || s.position !== it.sectionPosition) {
      s = { position: it.sectionPosition, title: it.sectionTitle, items: [] };
      out.push(s);
    }
    s.items.push(it);
  }
  return out;
}

/** "Câu 3." (+ điểm với câu tự luận) ghép vào đoạn đầu của nội dung câu. */
function questionBlock(q) {
  const label = h('strong', null, `Câu ${q.number}.`, q.kind === 'essay' ? ` (${formatScore(q.points)} điểm)` : '', ' ');
  const nodes = md(q.content);
  const first = nodes[0];
  const out = [];
  if (first && first.tagName === 'P') {
    first.prepend(label);
    out.push(...nodes);
  } else {
    out.push(h('p', null, label), ...nodes);
  }
  if (q.kind === 'single' || q.kind === 'multiple') {
    const cols = optionColumns(q.options);
    const cells = q.options.map((o, i) => h('td', null, h('p', null, h('strong', null, `${letter(i)}. `), ...mdInline(o.content))));
    const rows = [];
    for (let i = 0; i < cells.length; i += cols) rows.push(h('tr', null, cells.slice(i, i + cols)));
    out.push(h('table', { class: 'plain options' }, h('tbody', null, rows)));
  } else if (q.kind === 'true_false') {
    q.options.forEach((o, i) => out.push(h('p', { class: 'statement' }, h('strong', null, `${statementLetter(i)}) `), ...mdInline(o.content))));
  }
  return out;
}

/**
 * Một mã đề.
 * @param {{ code: string, items: object[] }} paper
 * @param {{ school?: string, examName?: string, subjectName?: string, minutes?: number|null }} header
 */
export function renderPaperDom(paper, header = {}) {
  const root = h('div', { class: 'paper' });
  const minutes = header.minutes ? `Thời gian làm bài: ${header.minutes} phút, không kể thời gian phát đề` : '';
  root.append(
    h(
      'table',
      { class: 'plain paper-head' },
      h(
        'tbody',
        null,
        h(
          'tr',
          null,
          h('td', null, h('p', { class: 'center' }, h('strong', null, (header.school || '').toUpperCase())), h('p', { class: 'center' }, `(Đề có ${paper.items.length} câu)`)),
          h(
            'td',
            null,
            h('p', { class: 'center' }, h('strong', null, (header.examName || 'ĐỀ KIỂM TRA').toUpperCase())),
            header.subjectName ? h('p', { class: 'center' }, h('strong', null, `Môn: ${header.subjectName}`)) : null,
            minutes ? h('p', { class: 'center' }, h('em', null, minutes)) : null,
          ),
        ),
      ),
    ),
    h(
      'table',
      { class: 'plain paper-name' },
      h(
        'tbody',
        null,
        h(
          'tr',
          null,
          h('td', null, h('p', null, 'Họ, tên thí sinh: ……………………………………'), h('p', null, 'Số báo danh: ………………')),
          h('td', null, h('p', { class: 'center' }, h('strong', null, `Mã đề ${paper.code}`))),
        ),
      ),
    ),
  );
  for (const s of sectionsOf(paper.items)) {
    root.append(h('h2', null, s.title));
    for (const q of s.items) root.append(...questionBlock(q));
  }
  root.append(h('p', { class: 'center' }, h('strong', null, '------ HẾT ------')));
  return root;
}

/** Tất cả mã đề nối nhau, mỗi mã sang trang mới. */
export function renderAllPapersDom(papers, header = {}) {
  const root = h('div', { class: 'papers' });
  papers.forEach((p, i) => {
    if (i > 0) root.append(h('div', { class: 'page-break' }));
    root.append(renderPaperDom(p, header));
  });
  return root;
}

/**
 * Bảng đáp án (câu × mã đề, từng phần) + hướng dẫn chấm các câu tự luận.
 * @param {{ code: string, items: object[] }[]} papers
 */
export function renderAnswerKeyDom(papers, header = {}) {
  const root = h('div', { class: 'answer-key' });
  root.append(
    h('h1', { class: 'plan-title' }, `ĐÁP ÁN — ${(header.examName || header.title || 'ĐỀ KIỂM TRA').toUpperCase()}`),
    header.subjectName ? h('p', { class: 'center' }, `Môn: ${header.subjectName}`) : null,
  );
  if (!papers.length) return root;
  const bySection = papers.map((p) => sectionsOf(p.items));
  bySection[0].forEach((section, si) => {
    root.append(h('h2', null, section.title));
    const count = Math.max(...bySection.map((ss) => ss[si]?.items.length ?? 0));
    const head = h('tr', null, h('th', null, 'Câu'), papers.map((p) => h('th', null, `Mã ${p.code}`)));
    const rows = Array.from({ length: count }, (_, qi) =>
      h(
        'tr',
        null,
        h('td', null, String(qi + 1)),
        bySection.map((ss) => {
          const q = ss[si]?.items[qi];
          return h('td', null, q ? answerCode(q) : '');
        }),
      ),
    );
    root.append(h('table', { class: 'key' }, h('thead', null, head), h('tbody', null, rows)));
  });

  // Hướng dẫn chấm tự luận (mỗi câu một lần, theo nội dung)
  const essays = new Map();
  for (const p of papers) for (const q of p.items) if (q.kind === 'essay' && !essays.has(q.content)) essays.set(q.content, q);
  if (essays.size) {
    root.append(h('h2', null, 'HƯỚNG DẪN CHẤM TỰ LUẬN'));
    let n = 0;
    for (const q of essays.values()) {
      n += 1;
      root.append(h('p', null, h('strong', null, `Câu tự luận ${n} (${formatScore(q.points)} điểm). `)), ...md(q.content));
      if (q.rubric?.length) {
        const total = Math.round(q.rubric.reduce((s, r) => s + Number(r.points), 0) * 100) / 100;
        if (Math.abs(total - q.points) > 1e-9) {
          root.append(h('p', null, h('em', null, `Tổng điểm các bước ${formatScore(total)}, quy về ${formatScore(q.points)} điểm của câu.`)));
        }
        root.append(
          h(
            'table',
            { class: 'key rubric' },
            h('thead', null, h('tr', null, h('th', null, 'Nội dung'), h('th', null, 'Điểm'))),
            h('tbody', null, q.rubric.map((s) => h('tr', null, h('td', null, h('p', null, ...mdInline(s.content))), h('td', null, formatScore(s.points))))),
          ),
        );
      }
      if (q.explanation) root.append(h('p', null, h('em', null, 'Lời giải mẫu:')), ...md(q.explanation));
    }
  }
  return root;
}

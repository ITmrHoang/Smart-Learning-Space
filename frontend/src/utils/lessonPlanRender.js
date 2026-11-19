/**
 * Dựng giáo án thành DOM thuần (không shadow DOM): dùng cho trang in và xuất Word (`utils/docx`).
 * Markdown + công thức dựng bằng `renderMarkdown` của H-UI → bản in / file Word giống hệt màn hình soạn.
 */
import { renderMarkdown } from '@itmrhoang/h-ui/hui-markdown';

import { ACTIVITY_PARTS, ACTIVITY_STEPS, PROBLEM_PARTS } from './lessonPlan';
import { letter, statementLetter } from './teaching';

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

/** Markdown → các nút (khối). Trống → không có gì. */
function md(text) {
  const value = String(text ?? '').trim();
  return value ? renderMarkdown(value) : [];
}

/** Markdown một dòng (trong ô bảng, đáp án ngắn). */
function mdInline(text) {
  const value = String(text ?? '').trim();
  return value ? renderMarkdown(value, { inline: true }) : [];
}

/** "a) Mục tiêu:" + nội dung (nhãn luôn hiện để giữ khung giáo án). */
function part(label, text) {
  return [h('p', { class: 'part-label' }, h('strong', null, `${label}:`)), ...md(text)];
}

function answerText(q) {
  if (q.kind === 'short') return (q.accepted ?? []).join('; ');
  if (q.kind === 'true_false') return q.options.map((o, i) => `${statementLetter(i)}) ${o.correct ? 'Đ' : 'S'}`).join('  ');
  return q.options.map((o, i) => (o.correct ? letter(i) : null)).filter(Boolean).join(', ');
}

/**
 * @param {object} plan - giáo án từ api (`title`, `info`, `blocks`, `attachments`…)
 * @param {{ subjectName?: string, gradeName?: string, questions?: Map<string, object> }} ctx
 * @returns {HTMLElement}
 */
export function renderPlanDom(plan, { subjectName = '', gradeName = '', questions = new Map() } = {}) {
  const info = plan.info ?? {};
  const root = h('div', { class: 'lesson-plan' });

  // Phần đầu: trường / tổ — giáo viên; ngày soạn, ngày dạy, lớp, thời lượng
  root.append(
    h(
      'table',
      { class: 'plan-head' },
      h(
        'tr',
        null,
        h('td', null, h('p', null, `Trường: ${info.school || '…'}`), h('p', null, `Tổ: ${info.group || '…'}`)),
        h('td', null, h('p', null, `Họ và tên giáo viên: ${info.teacherName || '…'}`)),
      ),
    ),
  );
  const dates = [
    info.preparedOn && `Ngày soạn: ${info.preparedOn}`,
    info.taughtOn && `Ngày dạy: ${info.taughtOn}`,
    info.className && `Lớp: ${info.className}`,
  ].filter(Boolean);
  if (dates.length) root.append(h('p', { class: 'plan-dates' }, dates.join('     ')));

  root.append(h('h1', { class: 'plan-title' }, `TÊN BÀI DẠY: ${String(plan.title || '').toUpperCase()}`));
  const sub = [subjectName && `Môn học: ${subjectName}`, gradeName && `lớp: ${gradeName}`].filter(Boolean).join('; ');
  const periods = info.periods ? `Thời gian thực hiện: ${info.periods}` : '';
  if (sub) root.append(h('p', { class: 'plan-sub' }, sub));
  if (periods) root.append(h('p', { class: 'plan-sub' }, `(${periods})`));

  let questionNo = 0;
  const answers = [];
  for (const b of plan.blocks ?? []) {
    switch (b.kind) {
      case 'section':
        if (b.title) root.append(h('h2', null, b.title));
        root.append(...md(b.body));
        break;
      case 'activity': {
        const minutes = Number.isFinite(b.minutes) && b.minutes > 0 ? ` (${b.minutes} phút)` : '';
        root.append(h('h3', null, `${b.title || 'Hoạt động'}${minutes}`));
        for (const [key, label] of ACTIVITY_PARTS) root.append(...part(label, b[key]));
        root.append(h('p', { class: 'part-label' }, h('strong', null, 'd) Tổ chức thực hiện:')));
        for (const [key, label] of ACTIVITY_STEPS) {
          const text = b.steps?.[key];
          root.append(h('p', { class: 'step-label' }, h('em', null, `${label}:`)), ...md(text));
        }
        break;
      }
      case 'two_columns': {
        if (b.title) root.append(h('h2', null, b.title));
        const head = h('tr', null, h('th', null, 'Hoạt động của giáo viên'), h('th', null, 'Hoạt động của học sinh'));
        const rows = (b.rows ?? []).map((r) => h('tr', null, h('td', null, ...md(r.teacher)), h('td', null, ...md(r.student))));
        root.append(h('table', { class: 'plan-table' }, h('thead', null, head), h('tbody', null, ...rows)));
        break;
      }
      case 'problem_type':
        root.append(h('h3', null, b.title || 'Dạng bài'));
        for (const [key, label] of PROBLEM_PARTS) root.append(...part(label, b[key]));
        break;
      case 'questions': {
        if (b.title) root.append(h('h3', null, b.title));
        for (const id of b.questionIds ?? []) {
          const q = questions.get(id);
          if (!q) continue;
          questionNo += 1;
          const label = h('strong', null, `Câu ${questionNo}. `);
          const body = md(q.content);
          // Ghép "Câu n." vào đoạn đầu cho gọn
          if (body[0]?.tagName === 'P') body[0].prepend(label);
          else root.append(h('p', null, label));
          root.append(...body);
          q.options.forEach((o, i) => {
            const mark = q.kind === 'true_false' ? `${statementLetter(i)}) ` : `${letter(i)}. `;
            root.append(h('p', { class: 'option' }, mark, ...mdInline(o.content)));
          });
          if (b.showAnswers) answers.push({ no: questionNo, q });
        }
        break;
      }
      default:
        break;
    }
  }

  if (answers.length) {
    root.append(h('h2', null, 'PHỤ LỤC: ĐÁP ÁN VÀ LỜI GIẢI'));
    for (const { no, q } of answers) {
      root.append(h('p', null, h('strong', null, `Câu ${no}: `), ...mdInline(answerText(q))));
      if (q.explanation?.trim()) root.append(...md(q.explanation));
    }
  }
  const files = plan.attachments ?? [];
  if (files.length) root.append(h('p', { class: 'plan-files' }, h('em', null, `Tài liệu kèm theo: ${files.map((f) => f.name).join('; ')}`)));
  return root;
}

/** Câu hỏi của các khối "phiếu học tập" (tải một lần, giữ trong Map). */
export async function loadPlanQuestions(plan, fetchQuestion, cache = new Map()) {
  const ids = [...new Set((plan.blocks ?? []).flatMap((b) => (b.kind === 'questions' ? (b.questionIds ?? []) : [])))];
  await Promise.all(
    ids
      .filter((id) => !cache.has(id))
      .map(async (id) => {
        try {
          cache.set(id, await fetchQuestion(id));
        } catch {
          /* câu đã bị xóa: bỏ qua */
        }
      }),
  );
  return cache;
}

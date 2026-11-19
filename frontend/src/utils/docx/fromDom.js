/**
 * DOM (do `renderMarkdown` của H-UI / bộ dựng giáo án tạo) → WordprocessingML (`<w:p>`, `<w:tbl>`…).
 * Ảnh / đồ thị: `media` (Map phần tử → { rId, cx, cy, id }) do `document.js` chuẩn bị trước (tải ảnh là việc async).
 */
import { escapeXml, mathToOmml } from './omml';

/** Bề rộng vùng chữ: A4 21 cm − lề trái 3 cm − lề phải 2 cm = 16 cm (twip). */
export const TEXT_WIDTH_TWIPS = 9071;

const BLOCK = new Set([
  'p', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'ul', 'ol', 'li', 'table', 'thead', 'tbody', 'tr', 'blockquote', 'pre', 'hr',
  'div', 'section', 'article', 'figure', 'hui-plot',
]);

const lower = (n) => (n.tagName || '').toLowerCase();
const isBlock = (n) => n.nodeType === 1 && BLOCK.has(lower(n));

function rPr(f) {
  const parts = [];
  if (f.code) parts.push('<w:rFonts w:ascii="Consolas" w:hAnsi="Consolas"/>');
  if (f.b) parts.push('<w:b/>');
  if (f.i) parts.push('<w:i/>');
  if (f.u) parts.push('<w:u w:val="single"/>');
  if (f.strike) parts.push('<w:strike/>');
  if (f.vert) parts.push(`<w:vertAlign w:val="${f.vert}"/>`);
  return parts.length ? `<w:rPr>${parts.join('')}</w:rPr>` : '';
}

function textRun(text, f) {
  if (!text) return '';
  // Xuống dòng trong khối code
  return text
    .split('\n')
    .map((line, i) => `<w:r>${rPr(f)}${i ? '<w:br/>' : ''}<w:t xml:space="preserve">${escapeXml(line)}</w:t></w:r>`)
    .join('');
}

/** Ảnh trong dòng (`wp:inline`). */
export function drawingRun(m) {
  return (
    `<w:r><w:drawing><wp:inline distT="0" distB="0" distL="0" distR="0"><wp:extent cx="${m.cx}" cy="${m.cy}"/>` +
    `<wp:docPr id="${m.id}" name="Hình ${m.id}"/><wp:cNvGraphicFramePr><a:graphicFrameLocks noChangeAspect="1"/></wp:cNvGraphicFramePr>` +
    '<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture"><pic:pic>' +
    `<pic:nvPicPr><pic:cNvPr id="${m.id}" name="image${m.id}"/><pic:cNvPicPr/></pic:nvPicPr>` +
    `<pic:blipFill><a:blip r:embed="${m.rId}"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>` +
    `<pic:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="${m.cx}" cy="${m.cy}"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></pic:spPr>` +
    '</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>'
  );
}

/** Các run của nội dung trong dòng. */
function runs(node, f, media) {
  if (node.nodeType === 3) return textRun(node.textContent.replace(/\s+/g, (s) => (f.code ? s : ' ')), f);
  if (node.nodeType !== 1) return '';
  const name = lower(node);
  const kids = () => [...node.childNodes].map((c) => runs(c, f2, media)).join('');
  let f2 = f;
  switch (name) {
    case 'br':
      return '<w:r><w:br/></w:r>';
    case 'math':
      return mathToOmml(node).replace('<m:oMathPara>', '').replace('</m:oMathPara>', '');
    case 'img':
    case 'hui-plot': {
      const m = media.get(node);
      return m ? drawingRun(m) : textRun(node.getAttribute('alt') ? `[${node.getAttribute('alt')}]` : '[ảnh]', f);
    }
    case 'strong':
    case 'b':
      f2 = { ...f, b: true };
      return kids();
    case 'em':
    case 'i':
      f2 = { ...f, i: true };
      return kids();
    case 'u':
      f2 = { ...f, u: true };
      return kids();
    case 'del':
    case 's':
    case 'strike':
      f2 = { ...f, strike: true };
      return kids();
    case 'code':
    case 'kbd':
      f2 = { ...f, code: true };
      return kids();
    case 'sup':
      f2 = { ...f, vert: 'superscript' };
      return kids();
    case 'sub':
      f2 = { ...f, vert: 'subscript' };
      return kids();
    default:
      return kids();
  }
}

/** Một đoạn văn. */
function paragraph(inlineNodes, ctx, { style, align, prefix, border } = {}) {
  const ppr = [];
  if (style) ppr.push(`<w:pStyle w:val="${style}"/>`);
  if (border) ppr.push('<w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="auto"/></w:pBdr>');
  if (ctx.indent) ppr.push(`<w:ind w:left="${ctx.indent}"${prefix ? ' w:hanging="360"' : ''}/>`);
  if (align) ppr.push(`<w:jc w:val="${align}"/>`);
  const f = { b: ctx.bold, i: ctx.italic, code: ctx.code };
  const body = (prefix ? textRun(prefix, f) : '') + inlineNodes.map((n) => runs(n, f, ctx.media)).join('');
  return `<w:p>${ppr.length ? `<w:pPr>${ppr.join('')}</w:pPr>` : ''}${body}</w:p>`;
}

function list(node, ctx) {
  const ordered = lower(node) === 'ol';
  const start = Number(node.getAttribute('start')) || 1;
  const out = [];
  [...node.children]
    .filter((li) => lower(li) === 'li')
    .forEach((li, i) => {
      const inner = { ...ctx, indent: (ctx.indent || 0) + 360 };
      const prefix = ordered ? `${start + i}.\t` : '•\t';
      const parts = blocks(li, inner);
      // Ghép ký hiệu đầu dòng vào đoạn đầu tiên của mục
      if (parts.length && parts[0].startsWith('<w:p>')) {
        out.push(parts[0].replace(/^<w:p>(<w:pPr>.*?<\/w:pPr>)?/, (all, ppr) => `<w:p>${ppr ?? ''}${textRun(prefix, {})}`));
        out.push(...parts.slice(1));
      } else {
        out.push(paragraph([], inner, { prefix }), ...parts);
      }
    });
  return out;
}

function tableXml(node, ctx) {
  const rows = [...node.querySelectorAll(':scope > tr, :scope > thead > tr, :scope > tbody > tr, :scope > tfoot > tr')];
  const cols = Math.max(1, ...rows.map((r) => [...r.children].length));
  const width = Math.floor((TEXT_WIDTH_TWIPS - (ctx.indent || 0)) / cols);
  const plain = node.classList.contains('plan-head');
  const border = plain ? 'none' : 'single';
  const borders = ['top', 'left', 'bottom', 'right', 'insideH', 'insideV']
    .map((s) => `<w:${s} w:val="${border}" w:sz="4" w:space="0" w:color="auto"/>`)
    .join('');
  const grid = Array.from({ length: cols }, () => `<w:gridCol w:w="${width}"/>`).join('');
  const body = rows
    .map((r) => {
      const cells = [...r.children].map((c) => {
        const cellCtx = { ...ctx, indent: 0, bold: ctx.bold || lower(c) === 'th' };
        const parts = blocks(c, cellCtx);
        // Ô phải có ít nhất một đoạn và kết thúc bằng đoạn
        if (!parts.length || !parts[parts.length - 1].startsWith('<w:p')) parts.push('<w:p/>');
        return `<w:tc><w:tcPr><w:tcW w:w="${width}" w:type="dxa"/></w:tcPr>${parts.join('')}</w:tc>`;
      });
      while (cells.length < cols) cells.push(`<w:tc><w:tcPr><w:tcW w:w="${width}" w:type="dxa"/></w:tcPr><w:p/></w:tc>`);
      return `<w:tr>${cells.join('')}</w:tr>`;
    })
    .join('');
  return (
    `<w:tbl><w:tblPr><w:tblW w:w="${width * cols}" w:type="dxa"/><w:tblBorders>${borders}</w:tblBorders>` +
    '<w:tblLayout w:type="fixed"/><w:tblCellMar><w:left w:w="100" w:type="dxa"/><w:right w:w="100" w:type="dxa"/></w:tblCellMar></w:tblPr>' +
    `<w:tblGrid>${grid}</w:tblGrid>${body}</w:tbl>`
  );
}

/**
 * Nội dung khối của một phần tử → mảng chuỗi XML (`<w:p>` / `<w:tbl>`).
 * @param {Element} node
 * @param {{ media: Map, indent?: number, bold?: boolean, italic?: boolean, code?: boolean }} ctx
 */
export function blocks(node, ctx) {
  const out = [];
  let pending = [];
  const flush = () => {
    if (pending.some((n) => n.nodeType === 1 || n.textContent.trim())) out.push(paragraph(pending, ctx));
    pending = [];
  };
  for (const child of node.childNodes) {
    if (!isBlock(child)) {
      pending.push(child);
      continue;
    }
    flush();
    const name = lower(child);
    if (/^h[1-6]$/.test(name)) {
      const level = Number(name[1]);
      const style = child.classList.contains('plan-title') ? 'Title' : `Heading${Math.min(level, 3)}`;
      out.push(paragraph([...child.childNodes], ctx, { style }));
    } else if (name === 'p') {
      const centered = child.classList.contains('plan-sub');
      out.push(paragraph([...child.childNodes], ctx, { align: centered ? 'center' : undefined }));
    } else if (name === 'ul' || name === 'ol') {
      out.push(...list(child, ctx));
    } else if (name === 'table') {
      out.push(tableXml(child, ctx));
    } else if (name === 'blockquote') {
      out.push(...blocks(child, { ...ctx, indent: (ctx.indent || 0) + 567, italic: true }));
    } else if (name === 'pre') {
      out.push(paragraph([document.createTextNode(child.textContent.replace(/\n$/, ''))], { ...ctx, code: true }));
    } else if (name === 'hr') {
      out.push(paragraph([], ctx, { border: true }));
    } else if (name === 'hui-plot') {
      out.push(paragraph([child], ctx, { align: 'center' }));
    } else if (name === 'div' && child.classList.contains('md-math')) {
      for (const m of child.querySelectorAll('math')) out.push(`<w:p><w:pPr><w:jc w:val="center"/></w:pPr>${mathToOmml(m)}</w:p>`);
    } else {
      out.push(...blocks(child, ctx));
    }
  }
  flush();
  return out;
}

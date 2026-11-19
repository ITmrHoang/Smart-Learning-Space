/**
 * MathML (do `hui-math` dựng từ LaTeX) → OMML (công thức của Word) — để file Word xuất ra có công thức sửa được,
 * không phải ảnh. Hỗ trợ đúng phần MathML mà hui-math sinh ra: phân số, căn, mũ / chỉ số, ngoặc co giãn,
 * ma trận / hệ (cases, aligned), dấu mũ / vector / gạch trên, giới hạn / tổng / tích phân có cận.
 */

export const escapeXml = (s) =>
  String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

const tag = (name) => (name || '').toLowerCase().replace(/^m:/, '');

/** Một "run" công thức; `plain` = chữ đứng (tên hàm sin, lim…, chữ trong \text{}). */
function run(text, plain = false) {
  if (text === '') return '';
  const rpr = plain ? '<m:rPr><m:sty m:val="p"/></m:rPr>' : '';
  return `<m:r>${rpr}<w:rPr><w:rFonts w:ascii="Cambria Math" w:hAnsi="Cambria Math"/></w:rPr><m:t xml:space="preserve">${escapeXml(text)}</m:t></m:r>`;
}

const elements = (node) => [...(node?.children ?? [])];

/** Đối số `m:e`, `m:num`… luôn có nội dung (Word không chịu ô rỗng ở một số chỗ). */
function wrap(name, inner) {
  return `<m:${name}>${inner || run('\u200b')}</m:${name}>`;
}

function isFence(node) {
  return tag(node?.tagName) === 'mo' && node.getAttribute('fence') === 'true';
}

function isLargeOp(node) {
  return tag(node?.tagName) === 'mo' && (node.getAttribute('largeop') === 'true' || /^[∑∏∐⋃⋂∫∬∭∮]$/.test(node.textContent.trim()));
}

const ACCENTS = { '→': '→', '←': '←', '^': '\u0302', '~': '\u0303', '˙': '\u0307', '¨': '\u0308', '⌒': '\u0311' };

function table(node, asEqArr) {
  const rows = elements(node).filter((r) => tag(r.tagName) === 'mtr');
  if (asEqArr) {
    // Hệ / căn trái: mỗi dòng một phương trình, các ô ghép bằng dấu cách
    return `<m:eqArr>${rows.map((r) => wrap('e', elements(r).map(convertChildren).join(run(' ')))).join('')}</m:eqArr>`;
  }
  return `<m:m>${rows.map((r) => `<m:mr>${elements(r).map((c) => wrap('e', convertChildren(c))).join('')}</m:mr>`).join('')}</m:m>`;
}

function convertChildren(node) {
  return elements(node).map(convert).join('');
}

/** Một nút MathML → OMML. */
export function convert(node) {
  const name = tag(node.tagName);
  const kids = elements(node);
  switch (name) {
    case 'math':
    case 'mstyle':
    case 'mpadded':
    case 'mphantom':
      return convertChildren(node);
    case 'semantics':
      return kids[0] ? convert(kids[0]) : '';
    case 'annotation':
    case 'annotation-xml':
      return '';
    case 'mrow': {
      // \left( … \right) → ngoặc co giãn của Word
      const first = kids[0];
      const last = kids[kids.length - 1];
      const opens = isFence(first);
      const closes = kids.length > 1 && isFence(last);
      if (opens || closes) {
        const inner = kids.slice(opens ? 1 : 0, closes ? -1 : undefined);
        const beg = opens ? first.textContent : '';
        const end = closes ? last.textContent : '';
        // Hệ phương trình: dấu { bên trái + bảng căn trái
        const only = inner.length === 1 ? inner[0] : null;
        const body =
          only && tag(only.tagName) === 'mtable'
            ? table(only, beg === '{' || only.getAttribute('columnalign') === 'left')
            : inner.map(convert).join('');
        return `<m:d><m:dPr><m:begChr m:val="${escapeXml(beg)}"/><m:endChr m:val="${escapeXml(end)}"/></m:dPr>${wrap('e', body)}</m:d>`;
      }
      return convertChildren(node);
    }
    case 'mi':
      return run(node.textContent, node.getAttribute('mathvariant') === 'normal' || node.textContent.length > 1);
    case 'mn':
      return run(node.textContent);
    case 'mo':
      return run(node.textContent);
    case 'mtext':
    case 'ms':
      return run(node.textContent, true);
    case 'mspace':
      return run(' ');
    case 'mfrac': {
      const bar = node.getAttribute('linethickness') === '0' ? '<m:fPr><m:type m:val="noBar"/></m:fPr>' : '';
      return `<m:f>${bar}${wrap('num', kids[0] ? convert(kids[0]) : '')}${wrap('den', kids[1] ? convert(kids[1]) : '')}</m:f>`;
    }
    case 'msqrt':
      return `<m:rad><m:radPr><m:degHide m:val="1"/></m:radPr><m:deg/>${wrap('e', convertChildren(node))}</m:rad>`;
    case 'mroot':
      return `<m:rad>${wrap('deg', kids[1] ? convert(kids[1]) : '')}${wrap('e', kids[0] ? convert(kids[0]) : '')}</m:rad>`;
    case 'msup':
      if (isLargeOp(kids[0])) return limit('limUpp', kids[0], kids[1]);
      return `<m:sSup>${wrap('e', convert(kids[0]))}${wrap('sup', kids[1] ? convert(kids[1]) : '')}</m:sSup>`;
    case 'msub':
      if (isLargeOp(kids[0]) || kids[0]?.textContent === 'lim') return limit('limLow', kids[0], kids[1]);
      return `<m:sSub>${wrap('e', convert(kids[0]))}${wrap('sub', kids[1] ? convert(kids[1]) : '')}</m:sSub>`;
    case 'msubsup':
      if (isLargeOp(kids[0])) return limit('limUpp', null, kids[2], limit('limLow', kids[0], kids[1]));
      return `<m:sSubSup>${wrap('e', convert(kids[0]))}${wrap('sub', convert(kids[1]))}${wrap('sup', convert(kids[2]))}</m:sSubSup>`;
    case 'munder': {
      const accent = node.getAttribute('accentunder') === 'true';
      if (accent) return `<m:bar><m:barPr><m:pos m:val="bot"/></m:barPr>${wrap('e', convert(kids[0]))}</m:bar>`;
      return limit('limLow', kids[0], kids[1]);
    }
    case 'mover': {
      if (node.getAttribute('accent') === 'true') {
        const chr = kids[1]?.textContent ?? '';
        if (chr === '‾' || chr === '¯') return `<m:bar><m:barPr><m:pos m:val="top"/></m:barPr>${wrap('e', convert(kids[0]))}</m:bar>`;
        return `<m:acc><m:accPr><m:chr m:val="${escapeXml(ACCENTS[chr] ?? chr)}"/></m:accPr>${wrap('e', convert(kids[0]))}</m:acc>`;
      }
      return limit('limUpp', kids[0], kids[1]);
    }
    case 'munderover':
      return limit('limUpp', null, kids[2], limit('limLow', kids[0], kids[1]));
    case 'mtable':
      return table(node, node.getAttribute('columnalign') === 'left');
    case 'mtr':
    case 'mtd':
      return convertChildren(node);
    default:
      return node.textContent ? run(node.textContent) : '';
  }
}

/** Cận dưới / trên của ∑, ∫, lim… (dùng limLow / limUpp, luôn có nội dung). */
function limit(kind, base, lim, baseXml) {
  const e = baseXml ?? (base ? convert(base) : '');
  const tagName = kind === 'limLow' ? 'm:limLow' : 'm:limUpp';
  return `<${tagName}>${wrap('e', e)}${wrap('lim', lim ? convert(lim) : '')}</${tagName}>`;
}

/** Phần tử <math> → `<m:oMath>` (trong dòng) hoặc `<m:oMathPara>` (khối, display="block"). */
export function mathToOmml(mathEl) {
  const inner = `<m:oMath>${convert(mathEl)}</m:oMath>`;
  return mathEl.getAttribute('display') === 'block' ? `<m:oMathPara>${inner}</m:oMathPara>` : inner;
}

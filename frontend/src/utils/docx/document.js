/**
 * Xuất file Word (.docx) từ DOM ở trình duyệt, không thư viện ngoài: Times New Roman 13, khổ A4,
 * lề trái 3 cm / phải 2 cm / trên, dưới 2 cm (theo thể thức văn bản hành chính), số trang ở chân trang.
 * Công thức → OMML (sửa được trong Word), ảnh nhúng vào file, đồ thị `hui-plot` vẽ ra PNG.
 *
 *   const blob = await exportDocx({ title: 'Giáo án…', root: element });
 */
import { drawPlot, fitView, parsePlot } from '@itmrhoang/h-ui/hui-plot';

import { TEXT_WIDTH_TWIPS, blocks } from './fromDom';
import { escapeXml } from './omml';
import { zip } from './zip';

const NS =
  'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" ' +
  'xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" ' +
  'xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" ' +
  'xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" ' +
  'xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" ' +
  'xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"';
const XML = '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n';
const EMU_PER_PX = 9525;
/** Bề rộng tối đa của ảnh = bề rộng vùng chữ (twip → EMU). */
const MAX_CX = TEXT_WIDTH_TWIPS * 635;

const CONTENT_TYPES = `${XML}<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Default Extension="jpeg" ContentType="image/jpeg"/>
<Default Extension="gif" ContentType="image/gif"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
<Override PartName="/word/footer1.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml"/>
<Override PartName="/docProps/core.xml" ContentType="application/vnd.openxmlformats-package.core-properties+xml"/>
</Types>`;

const ROOT_RELS = `${XML}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="docProps/core.xml"/>
</Relationships>`;

const heading = (id, name, size, before, outline, extra = '') =>
  `<w:style w:type="paragraph" w:styleId="${id}"><w:name w:val="${name}"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:qFormat/>` +
  `<w:pPr><w:keepNext/><w:spacing w:before="${before}" w:after="80"/>${outline != null ? `<w:outlineLvl w:val="${outline}"/>` : ''}${extra}</w:pPr>` +
  `<w:rPr><w:b/><w:sz w:val="${size}"/><w:szCs w:val="${size}"/></w:rPr></w:style>`;

const STYLES = `${XML}<w:styles ${NS}>
<w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Times New Roman" w:hAnsi="Times New Roman" w:eastAsia="Times New Roman" w:cs="Times New Roman"/><w:sz w:val="26"/><w:szCs w:val="26"/><w:lang w:val="vi-VN"/></w:rPr></w:rPrDefault>
<w:pPrDefault><w:pPr><w:spacing w:after="60" w:line="288" w:lineRule="auto"/><w:jc w:val="both"/></w:pPr></w:pPrDefault></w:docDefaults>
<w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/><w:qFormat/></w:style>
${heading('Title', 'Title', 28, 240, null, '<w:jc w:val="center"/>')}
${heading('Heading1', 'heading 1', 28, 240, 0)}
${heading('Heading2', 'heading 2', 26, 180, 1)}
${heading('Heading3', 'heading 3', 26, 120, 2)}
</w:styles>`;

const FOOTER = `${XML}<w:ftr ${NS}><w:p><w:pPr><w:jc w:val="center"/></w:pPr>` +
  '<w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText xml:space="preserve"> PAGE </w:instrText></w:r>' +
  '<w:r><w:fldChar w:fldCharType="separate"/></w:r><w:r><w:t>1</w:t></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r></w:p></w:ftr>';

const SECT =
  '<w:sectPr><w:footerReference w:type="default" r:id="rIdFooter"/><w:pgSz w:w="11906" w:h="16838"/>' +
  '<w:pgMar w:top="1134" w:right="1134" w:bottom="1134" w:left="1701" w:header="567" w:footer="567" w:gutter="0"/></w:sectPr>';

function core(title) {
  const now = new Date().toISOString().replace(/\.\d+Z$/, 'Z');
  return `${XML}<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:dcterms="http://purl.org/dc/terms/" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">` +
    `<dc:title>${escapeXml(title)}</dc:title><dc:creator>Smart Learning Space</dc:creator>` +
    `<dcterms:created xsi:type="dcterms:W3CDTF">${now}</dcterms:created><dcterms:modified xsi:type="dcterms:W3CDTF">${now}</dcterms:modified>` +
    '</cp:coreProperties>';
}

const canvasBlob = (canvas) => new Promise((resolve) => canvas.toBlob(resolve, 'image/png'));

/** Ảnh → { bytes, ext, width, height } (đổi sang PNG nếu Word không đọc được, ví dụ WebP). */
async function loadImage(src) {
  const res = await fetch(src);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  let blob = await res.blob();
  const bitmap = await createImageBitmap(blob);
  let ext = { 'image/png': 'png', 'image/jpeg': 'jpeg', 'image/gif': 'gif' }[blob.type];
  if (!ext) {
    const canvas = document.createElement('canvas');
    canvas.width = bitmap.width;
    canvas.height = bitmap.height;
    canvas.getContext('2d').drawImage(bitmap, 0, 0);
    blob = await canvasBlob(canvas);
    ext = 'png';
  }
  return { bytes: new Uint8Array(await blob.arrayBuffer()), ext, width: bitmap.width, height: bitmap.height };
}

/** Đồ thị → PNG (vẽ lại bằng lõi của hui-plot, 2x cho nét). */
export async function plotImage(value, { width = 640, height = 360, scale = 2 } = {}) {
  const canvas = document.createElement('canvas');
  canvas.width = width * scale;
  canvas.height = height * scale;
  const ctx = canvas.getContext('2d');
  ctx.scale(scale, scale);
  const plot = parsePlot(value);
  drawPlot(ctx, plot, fitView(plot.view, height / width), { w: width, h: height, font: 13, background: '#ffffff' });
  const blob = await canvasBlob(canvas);
  return { bytes: new Uint8Array(await blob.arrayBuffer()), ext: 'png', width, height };
}

/**
 * Dựng file .docx (bytes).
 * @param {{ title: string, root: Element }} opts
 * @returns {Promise<Uint8Array>}
 */
export async function buildDocx({ title, root }) {
  const media = new Map();
  const files = [];
  const rels = [];
  let n = 0;
  for (const el of root.querySelectorAll('img, hui-plot')) {
    try {
      const img = el.tagName.toLowerCase() === 'hui-plot' ? await plotImage(el.getAttribute('value') ?? el.value ?? '') : await loadImage(el.src);
      n += 1;
      const scale = Math.min(1, MAX_CX / (img.width * EMU_PER_PX));
      const name = `image${n}.${img.ext}`;
      files.push({ name: `word/media/${name}`, data: img.bytes });
      rels.push(`<Relationship Id="rIdImg${n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/${name}"/>`);
      media.set(el, {
        rId: `rIdImg${n}`,
        id: n,
        cx: Math.round(img.width * EMU_PER_PX * scale),
        cy: Math.round(img.height * EMU_PER_PX * scale),
      });
    } catch {
      // Ảnh không tải được: ghi "[ảnh]" thay vì làm hỏng cả file
    }
  }
  const body = blocks(root, { media }).join('');
  const documentXml = `${XML}<w:document ${NS}><w:body>${body}${SECT}</w:body></w:document>`;
  const docRels = `${XML}<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
<Relationship Id="rIdFooter" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footer" Target="footer1.xml"/>
${rels.join('\n')}
</Relationships>`;
  return zip([
    { name: '[Content_Types].xml', data: CONTENT_TYPES },
    { name: '_rels/.rels', data: ROOT_RELS },
    { name: 'docProps/core.xml', data: core(title) },
    { name: 'word/document.xml', data: documentXml },
    { name: 'word/styles.xml', data: STYLES },
    { name: 'word/footer1.xml', data: FOOTER },
    { name: 'word/_rels/document.xml.rels', data: docRels },
    ...files,
  ]);
}

/** File Word để tải về. */
export async function exportDocx(opts) {
  const bytes = await buildDocx(opts);
  return new Blob([bytes], { type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' });
}

/** Tải Blob về máy với tên file. */
export function downloadBlob(blob, fileName) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = fileName;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

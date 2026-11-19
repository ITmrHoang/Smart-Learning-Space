import { renderMarkdown } from '@itmrhoang/h-ui/hui-markdown';
import { latexToMathML } from '@itmrhoang/h-ui/hui-math';
import { describe, expect, it } from 'vitest';

import { buildDocx } from './document';
import { blocks } from './fromDom';
import { mathToOmml } from './omml';
import { crc32, readZip, zip } from './zip';

const W_NS = 'xmlns:w="w" xmlns:m="m" xmlns:r="r" xmlns:wp="wp" xmlns:a="a" xmlns:pic="pic"';
/** XML phải hợp lệ (đóng / mở thẻ đúng) — Word không mở file hỏng. */
function wellFormed(xml) {
  const doc = new DOMParser().parseFromString(xml, 'application/xml');
  return !doc.getElementsByTagName('parsererror').length;
}
const omml = (tex, display = false) => mathToOmml(latexToMathML(tex, { display }));

describe('docx / zip', () => {
  it('CRC32 chuẩn, ghi rồi đọc lại đủ file (tên tiếng Việt)', () => {
    expect(crc32(new TextEncoder().encode('123456789'))).toBe(0xcbf43926);
    const bytes = zip([
      { name: 'a.txt', data: 'xin chào' },
      { name: 'thư mục/ảnh.bin', data: new Uint8Array([1, 2, 3]) },
    ]);
    const files = readZip(bytes);
    expect(new TextDecoder().decode(files.get('a.txt'))).toBe('xin chào');
    expect([...files.get('thư mục/ảnh.bin')]).toEqual([1, 2, 3]);
  });
});

describe('docx / công thức (MathML → OMML)', () => {
  it('phân số, căn, mũ, chỉ số', () => {
    const f = omml('\\frac{1}{2}');
    expect(f).toMatch(/<m:f><m:num>.*1.*<\/m:num><m:den>.*2.*<\/m:den><\/m:f>/);
    expect(omml('\\sqrt{x}')).toContain('<m:degHide m:val="1"/>');
    expect(omml('\\sqrt[3]{x}')).toMatch(/<m:deg>.*3.*<\/m:deg>/);
    expect(omml('x^2')).toContain('<m:sSup>');
    expect(omml('a_n')).toContain('<m:sSub>');
    expect(wellFormed(`<r ${W_NS}>${f}</r>`)).toBe(true);
  });

  it('ngoặc co giãn, hệ phương trình, ma trận, vector, giới hạn, tổng', () => {
    expect(omml('\\left( x+1 \\right)')).toMatch(/<m:begChr m:val="\("\/><m:endChr m:val="\)"\/>/);
    const sys = omml('\\begin{cases} x + y = 5 \\\\ x - y = 1 \\end{cases}');
    expect(sys).toContain('<m:begChr m:val="{"/><m:endChr m:val=""/>');
    expect(sys.match(/<m:eqArr>/g)).toHaveLength(1);
    expect(omml('\\begin{pmatrix} 1 & 2 \\\\ 3 & 4 \\end{pmatrix}').match(/<m:mr>/g)).toHaveLength(2);
    expect(omml('\\vec{AB}')).toContain('<m:acc>');
    expect(omml('\\overline{AB}')).toContain('<m:pos m:val="top"/>');
    const lim = omml('\\lim_{x \\to 0} f(x)');
    expect(lim).toContain('<m:limLow>');
    expect(lim).toContain('<m:sty m:val="p"/>');
    expect(omml('\\sum_{k=1}^{n} k', true)).toContain('<m:limUpp>');
    expect(omml('x', true)).toMatch(/^<m:oMathPara>/);
    for (const tex of ['\\frac{\\sqrt{2}}{x^2+1}', '\\begin{cases} x>0 \\\\ y<1 \\end{cases}', '\\widehat{ABC} = 90^\\circ']) {
      expect(wellFormed(`<r ${W_NS}>${omml(tex)}</r>`), tex).toBe(true);
    }
  });
});

describe('docx / DOM → Word', () => {
  const md = '# Tiêu đề\n\nĐoạn **đậm**, *nghiêng* và $x^2 + 1$.\n\n- ý một\n- ý hai\n\n1. bước 1\n2. bước 2\n\n| A | B |\n|---|---|\n| 1 | $\\sqrt 2$ |\n\n$$\\frac{a}{b}$$';

  it('tiêu đề, chữ đậm, danh sách, bảng, công thức trong dòng / khối', () => {
    const root = document.createElement('div');
    root.append(...renderMarkdown(md));
    const xml = blocks(root, { media: new Map() }).join('');
    expect(xml).toContain('<w:pStyle w:val="Heading1"/>');
    expect(xml).toContain('<w:b/>');
    expect(xml).toContain('<w:i/>');
    expect(xml).toContain('•\t');
    expect(xml).toContain('2.\t');
    expect(xml).toContain('<w:tbl>');
    expect(xml).toContain('<m:oMath>');
    expect(xml).toContain('<m:oMathPara>');
    expect(wellFormed(`<r ${W_NS}>${xml}</r>`)).toBe(true);
  });

  it('file .docx đủ phần, XML hợp lệ, tiếng Việt giữ nguyên', async () => {
    const root = document.createElement('div');
    root.append(...renderMarkdown(md));
    const files = readZip(await buildDocx({ title: 'Giáo án thử', root }));
    for (const part of ['[Content_Types].xml', '_rels/.rels', 'word/document.xml', 'word/styles.xml', 'word/footer1.xml', 'word/_rels/document.xml.rels', 'docProps/core.xml']) {
      expect(files.has(part), part).toBe(true);
      expect(wellFormed(new TextDecoder().decode(files.get(part))), part).toBe(true);
    }
    const doc = new TextDecoder().decode(files.get('word/document.xml'));
    expect(doc).toContain('Tiêu đề');
    expect(doc).toContain('<w:pgSz w:w="11906" w:h="16838"/>');
    expect(new TextDecoder().decode(files.get('docProps/core.xml'))).toContain('Giáo án thử');
  });
});

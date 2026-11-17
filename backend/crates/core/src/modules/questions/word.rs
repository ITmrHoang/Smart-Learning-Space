//! Nhập câu hỏi từ file Word (.docx) — đề soạn theo cách giáo viên vẫn gõ:
//!
//! ```text
//! PHẦN I. Câu trắc nghiệm nhiều phương án lựa chọn      ← đổi loại câu mặc định cho các câu sau
//! Câu 1 (NB). Giá trị của √16 là                        ← mức độ trong ngoặc: NB / TH / VD / VDC
//! A. 2      B. 4      C. 8      D. 16                    ← nhiều phương án một dòng (tab / dấu cách)
//! Đáp án: B                                              ← hoặc gạch chân / tô đỏ / tô nền phương án đúng, hoặc *B.
//! Lời giải: …                                            ← "Lời giải", "Hướng dẫn giải", "Giải:" tới hết câu
//! ```
//!
//! - Đúng sai: các ý `a)` `b)` `c)` `d)`; "Đáp án: Đ S Đ S" hoặc gạch chân / tô đỏ các ý đúng.
//! - Trả lời ngắn: không có phương án, "Đáp án: 0,5; 1/2".
//! - Công thức Equation của Word (OMML) → LaTeX `$…$`; gõ sẵn `$…$` cũng được.
//! - Ảnh (PNG / JPG / GIF) → ghi tạm `![](docx-image:N)`, service lưu lên kho rồi thay bằng link thật.
//! - Công thức MathType (OLE) và ảnh WMF / EMF chưa đọc được → để chỗ trống kèm cảnh báo.
//! - Số câu / A. B. C. đánh tự động (Numbering của Word) cũng nhận.

use std::{
    collections::HashMap,
    io::{Cursor, Read, Write},
};

use quick_xml::{
    Reader, XmlVersion,
    events::{BytesStart, Event},
};
use uuid::Uuid;
use zip::{ZipArchive, ZipWriter, write::SimpleFileOptions};

use super::{
    dto::{ImportItem, OptionInput},
    import::{MAX_ROWS, parse_letters, parse_level, parse_truth},
    model::QuestionKind,
};
use crate::{
    error::{AppError, AppResult},
    modules::catalog::model::QuestionLevel,
    slug::fold,
};

/// Link tạm của ảnh trong nội dung: `![](docx-image:3)`.
pub const IMAGE_SCHEME: &str = "docx-image:";
const MATHTYPE: &str = "[công thức MathType]";
const UNSUPPORTED_IMAGE: &str = "[ảnh WMF/EMF]";
const MAX_XML_BYTES: u64 = 40 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_IMAGES: usize = 300;
const MAX_DEPTH: usize = 256;

/// Ảnh trong file Word (theo thứ tự xuất hiện).
pub struct WordImage {
    pub name: String,
    /// `None`: định dạng trình duyệt không hiện được (WMF / EMF…) hoặc quá lớn.
    pub content_type: Option<&'static str>,
    pub bytes: Vec<u8>,
}

pub struct WordImport {
    pub items: Vec<ImportItem>,
    pub images: Vec<WordImage>,
}

fn invalid_file(detail: impl std::fmt::Display) -> AppError {
    tracing::info!(error = %detail, "file Word không đọc được");
    AppError::bad_request("INVALID_FILE", "Không đọc được file Word. Hãy dùng file .docx (Word 2007 trở lên).")
}

// =====================================================================
// XML → cây
// =====================================================================

#[derive(Debug, Default)]
struct Node {
    /// Tên thẻ kèm tiền tố (`w:p`, `m:f`); rỗng = nút chữ.
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    text: String,
}

impl Node {
    fn is_text(&self) -> bool {
        self.name.is_empty()
    }

    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }

    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// Thuộc tính `val` (`w:val`, `m:val`).
    fn val(&self) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k.rsplit(':').next() == Some("val")).map(|(_, v)| v.as_str())
    }

    /// Phần tử con cháu đầu tiên có tên `name`.
    fn find(&self, name: &str) -> Option<&Node> {
        self.children.iter().find_map(|c| if c.name == name { Some(c) } else { c.find(name) })
    }

    fn text_content(&self) -> String {
        if self.is_text() {
            return self.text.clone();
        }
        self.children.iter().map(Node::text_content).collect()
    }
}

fn element(e: &BytesStart) -> Node {
    Node {
        name: e.name().as_ref().to_string(),
        attrs: e
            .attributes()
            .flatten()
            .map(|a| {
                (
                    a.key.as_ref().to_string(),
                    a.normalized_value(XmlVersion::Implicit1_0).map(|v| v.into_owned()).unwrap_or_default(),
                )
            })
            .collect(),
        ..Default::default()
    }
}

fn push_child(stack: &mut [Node], node: Node) {
    if let Some(top) = stack.last_mut() {
        top.children.push(node);
    }
}

fn push_text(stack: &mut [Node], text: &str) {
    let Some(top) = stack.last_mut() else { return };
    match top.children.last_mut() {
        Some(last) if last.is_text() => last.text.push_str(text),
        _ => top.children.push(Node { text: text.to_string(), ..Default::default() }),
    }
}

fn parse_xml(src: &str) -> AppResult<Node> {
    let mut reader = Reader::from_str(src);
    let mut stack = vec![Node { name: "#root".into(), ..Default::default() }];
    loop {
        match reader.read_event().map_err(invalid_file)? {
            Event::Start(e) => {
                if stack.len() > MAX_DEPTH {
                    return Err(invalid_file("XML lồng quá sâu"));
                }
                stack.push(element(&e));
            }
            Event::Empty(e) => {
                let node = element(&e);
                push_child(&mut stack, node);
            }
            Event::End(_) => {
                let node = stack.pop().ok_or_else(|| invalid_file("thẻ đóng thừa"))?;
                if stack.is_empty() {
                    return Err(invalid_file("thẻ đóng thừa"));
                }
                push_child(&mut stack, node);
            }
            Event::Text(t) => push_text(&mut stack, &t.xml10_content()),
            Event::GeneralRef(r) => {
                let name = r.xml10_content();
                let ch = match r.resolve_char_ref() {
                    Ok(Some(c)) => c.to_string(),
                    _ => match name.as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => "",
                    }
                    .to_string(),
                };
                push_text(&mut stack, &ch);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err(invalid_file("thiếu thẻ đóng"));
    }
    Ok(stack.pop().unwrap_or_default())
}

// =====================================================================
// Đọc file .docx
// =====================================================================

type Zip<'a> = ZipArchive<Cursor<&'a [u8]>>;

fn read_entry(zip: &mut Zip<'_>, name: &str, max: u64) -> AppResult<Option<Vec<u8>>> {
    let file = match zip.by_name(name) {
        Ok(f) => f,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(invalid_file(e)),
    };
    let mut out = Vec::new();
    file.take(max + 1).read_to_end(&mut out).map_err(invalid_file)?;
    if out.len() as u64 > max {
        return Err(AppError::bad_request("FILE_TOO_LARGE", format!("Phần \"{name}\" trong file Word quá lớn.")));
    }
    Ok(Some(out))
}

fn read_xml(zip: &mut Zip<'_>, name: &str) -> AppResult<Option<Node>> {
    match read_entry(zip, name, MAX_XML_BYTES)? {
        Some(bytes) => {
            let text = String::from_utf8(bytes).map_err(invalid_file)?;
            Ok(Some(parse_xml(&text)?))
        }
        None => Ok(None),
    }
}

/// `rId` → đường dẫn trong file zip (chỉ quan hệ ảnh).
fn image_rels(rels: Option<&Node>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(root) = rels.and_then(|r| r.child("Relationships")) else { return out };
    for rel in root.children_named("Relationship") {
        let (Some(id), Some(target), Some(kind)) = (rel.attr("Id"), rel.attr("Target"), rel.attr("Type")) else {
            continue;
        };
        if !kind.ends_with("/image") || rel.attr("TargetMode") == Some("External") {
            continue;
        }
        let path = match target.strip_prefix('/') {
            Some(abs) => abs.to_string(),
            None => {
                let mut parts: Vec<&str> = vec!["word"];
                for seg in target.split('/') {
                    match seg {
                        ".." => {
                            parts.pop();
                        }
                        "." | "" => {}
                        s => parts.push(s),
                    }
                }
                parts.join("/")
            }
        };
        out.insert(id.to_string(), path);
    }
    out
}

fn image_type(path: &str) -> Option<&'static str> {
    match path.rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("png") => Some("image/png"),
        Some("jpg" | "jpeg") => Some("image/jpeg"),
        Some("gif") => Some("image/gif"),
        Some("webp") => Some("image/webp"),
        _ => None,
    }
}

// ---------- Đánh số tự động (numbering.xml) ----------

#[derive(Clone)]
struct Level {
    start: u32,
    format: String,
    text: String,
}

#[derive(Default)]
struct Numbering {
    /// numId → (abstractNumId, cấp có đặt lại số đầu)
    nums: HashMap<String, (String, Vec<usize>)>,
    abstracts: HashMap<String, Vec<Level>>,
    counters: HashMap<String, [u32; 9]>,
    seen: Vec<String>,
}

impl Numbering {
    fn load(root: Option<&Node>) -> Self {
        let mut out = Self::default();
        let Some(numbering) = root.and_then(|r| r.child("w:numbering")) else { return out };
        for abs in numbering.children_named("w:abstractNum") {
            let Some(id) = abs.attr("w:abstractNumId") else { continue };
            let mut levels = vec![Level { start: 1, format: "decimal".into(), text: String::new() }; 9];
            for lvl in abs.children_named("w:lvl") {
                let Some(i) = lvl.attr("w:ilvl").and_then(|v| v.parse::<usize>().ok()).filter(|i| *i < 9) else {
                    continue;
                };
                levels[i] = Level {
                    start: lvl.child("w:start").and_then(Node::val).and_then(|v| v.parse().ok()).unwrap_or(1),
                    format: lvl.child("w:numFmt").and_then(Node::val).unwrap_or("decimal").to_string(),
                    text: lvl.child("w:lvlText").and_then(Node::val).unwrap_or("").to_string(),
                };
            }
            out.abstracts.insert(id.to_string(), levels);
        }
        for num in numbering.children_named("w:num") {
            let (Some(id), Some(abs)) = (num.attr("w:numId"), num.child("w:abstractNumId").and_then(Node::val)) else {
                continue;
            };
            let restarts = num
                .children_named("w:lvlOverride")
                .filter(|o| o.child("w:startOverride").is_some())
                .filter_map(|o| o.attr("w:ilvl").and_then(|v| v.parse().ok()))
                .collect();
            out.nums.insert(id.to_string(), (abs.to_string(), restarts));
        }
        out
    }

    /// Chữ đánh số của đoạn (`Câu 3.`, `B.`…); `None` khi không đánh số hoặc là gạch đầu dòng.
    fn label(&mut self, ppr: Option<&Node>) -> Option<String> {
        let num_pr = ppr?.child("w:numPr")?;
        let num_id = num_pr.child("w:numId").and_then(Node::val)?.to_string();
        let ilvl: usize = num_pr.child("w:ilvl").and_then(Node::val).and_then(|v| v.parse().ok()).unwrap_or(0).min(8);
        let (abs, restarts) = self.nums.get(&num_id)?.clone();
        let levels = self.abstracts.get(&abs)?.clone();
        let counters = self.counters.entry(abs.clone()).or_insert([0; 9]);
        if !self.seen.contains(&num_id) {
            self.seen.push(num_id);
            for l in restarts.into_iter().filter(|l| *l < 9) {
                counters[l] = 0;
            }
        }
        counters[ilvl] = if counters[ilvl] == 0 { levels[ilvl].start } else { counters[ilvl] + 1 };
        for c in counters.iter_mut().skip(ilvl + 1) {
            *c = 0;
        }
        let level = &levels[ilvl];
        if matches!(level.format.as_str(), "bullet" | "none") || level.text.is_empty() {
            return None;
        }
        let mut text = level.text.clone();
        for (k, lv) in levels.iter().enumerate().take(ilvl + 1) {
            let n = counters[k].max(lv.start);
            text = text.replace(&format!("%{}", k + 1), &format_number(n, &lv.format));
        }
        Some(text)
    }
}

fn format_number(n: u32, format: &str) -> String {
    let letters = |upper: bool| {
        let base = if upper { b'A' } else { b'a' };
        let letter = (base + ((n.max(1) - 1) % 26) as u8) as char;
        letter.to_string().repeat(((n.max(1) - 1) / 26 + 1) as usize)
    };
    let roman = |upper: bool| {
        let mut v = n;
        let mut s = String::new();
        for (value, sym) in [
            (1000, "m"),
            (900, "cm"),
            (500, "d"),
            (400, "cd"),
            (100, "c"),
            (90, "xc"),
            (50, "l"),
            (40, "xl"),
            (10, "x"),
            (9, "ix"),
            (5, "v"),
            (4, "iv"),
            (1, "i"),
        ] {
            while v >= value {
                s.push_str(sym);
                v -= value;
            }
        }
        if upper { s.to_uppercase() } else { s }
    };
    match format {
        "upperLetter" => letters(true),
        "lowerLetter" => letters(false),
        "upperRoman" => roman(true),
        "lowerRoman" => roman(false),
        "decimalZero" => format!("{n:02}"),
        _ => n.to_string(),
    }
}

// ---------- Đoạn văn → các dòng ----------

/// Một mẩu chữ cùng định dạng. `atomic`: công thức / ảnh (không cắt, không định dạng thêm).
#[derive(Clone, Default)]
struct Piece {
    text: String,
    bold: bool,
    italic: bool,
    /// Gạch chân / tô đỏ / tô nền — cách giáo viên đánh dấu đáp án đúng.
    marked: bool,
    atomic: bool,
}

type Line = Vec<Piece>;

#[derive(Clone, Copy, Default)]
struct RunStyle {
    bold: bool,
    italic: bool,
    marked: bool,
    sup: bool,
    sub: bool,
}

fn on(node: Option<&Node>) -> bool {
    node.is_some_and(|n| !matches!(n.val(), Some("0" | "false" | "off" | "none")))
}

fn run_style(rpr: Option<&Node>) -> RunStyle {
    let Some(rpr) = rpr else { return RunStyle::default() };
    let red = rpr.child("w:color").and_then(Node::val).is_some_and(|hex| {
        let v = u32::from_str_radix(hex, 16).unwrap_or(0);
        let (r, g, b) = ((v >> 16) & 0xFF, (v >> 8) & 0xFF, v & 0xFF);
        hex.len() == 6 && r >= 0xB0 && g < 0x70 && b < 0x70
    });
    let shaded = rpr
        .child("w:shd")
        .and_then(|s| s.attr("w:fill"))
        .is_some_and(|f| !matches!(f.to_ascii_lowercase().as_str(), "auto" | "ffffff" | ""));
    let vert = rpr.child("w:vertAlign").and_then(Node::val);
    RunStyle {
        bold: on(rpr.child("w:b")),
        italic: on(rpr.child("w:i")),
        marked: on(rpr.child("w:u")) || red || on(rpr.child("w:highlight")) || shaded,
        sup: vert == Some("superscript"),
        sub: vert == Some("subscript"),
    }
}

/// Chỉ số / số mũ gõ bằng định dạng chữ (x², H₂O): đổi sang ký tự Unicode nếu được, không thì `^(…)`.
fn script_text(text: &str, sup: bool) -> String {
    const SUP: [(char, char); 14] = [
        ('0', '⁰'),
        ('1', '¹'),
        ('2', '²'),
        ('3', '³'),
        ('4', '⁴'),
        ('5', '⁵'),
        ('6', '⁶'),
        ('7', '⁷'),
        ('8', '⁸'),
        ('9', '⁹'),
        ('+', '⁺'),
        ('-', '⁻'),
        ('n', 'ⁿ'),
        ('(', '⁽'),
    ];
    const SUB: [(char, char); 12] = [
        ('0', '₀'),
        ('1', '₁'),
        ('2', '₂'),
        ('3', '₃'),
        ('4', '₄'),
        ('5', '₅'),
        ('6', '₆'),
        ('7', '₇'),
        ('8', '₈'),
        ('9', '₉'),
        ('+', '₊'),
        ('-', '₋'),
    ];
    let table: &[(char, char)] = if sup { &SUP } else { &SUB };
    let mapped: Option<String> = text.chars().map(|c| table.iter().find(|(k, _)| *k == c).map(|(_, v)| *v)).collect();
    mapped.unwrap_or_else(|| format!("{}({text})", if sup { '^' } else { '_' }))
}

/// Ký tự font Symbol (w:sym) hay dùng trong đề.
fn symbol_char(font: &str, code: &str) -> Option<char> {
    let code = u32::from_str_radix(code, 16).ok()?;
    let code = if code >= 0xF000 { code - 0xF000 } else { code };
    if !fold(font).contains("symbol") {
        return None;
    }
    Some(match code {
        0x61 => 'α',
        0x62 => 'β',
        0x63 => 'χ',
        0x64 => 'δ',
        0x65 => 'ε',
        0x66 => 'φ',
        0x67 => 'γ',
        0x68 => 'η',
        0x6C => 'λ',
        0x6D => 'μ',
        0x70 => 'π',
        0x71 => 'θ',
        0x72 => 'ρ',
        0x73 => 'σ',
        0x74 => 'τ',
        0x77 => 'ω',
        0x44 => 'Δ',
        0x46 => 'Φ',
        0x50 => 'Π',
        0x53 => 'Σ',
        0x57 => 'Ω',
        0x2D => '−',
        0x5E => '⊥',
        0xA3 => '≤',
        0xB3 => '≥',
        0xB9 => '≠',
        0xB1 => '±',
        0xB4 => '×',
        0xB8 => '÷',
        0xBB => '≈',
        0xA5 => '∞',
        0xB0 => '°',
        0xD6 => '√',
        0xAE => '→',
        0xDE => '⇒',
        0xDB => '⇔',
        0xCE => '∈',
        0xCF => '∉',
        0xC6 => '∅',
        0xC7 => '∩',
        0xC8 => '∪',
        0xCC => '⊂',
        0xD0 => '∠',
        0xBA => '≡',
        0xA2 => '′',
        0xD7 => '⋅',
        0xA4 => '/',
        c if (0x20..0x7F).contains(&c) => char::from_u32(c)?,
        _ => return None,
    })
}

fn is_mathtype(object: &Node) -> bool {
    object
        .find("o:OLEObject")
        .and_then(|o| o.attr("ProgID"))
        .is_some_and(|p| p.starts_with("Equation.") || p.contains("MathType"))
}

struct Walker<'a> {
    rels: &'a HashMap<String, String>,
    numbering: Numbering,
    /// Đường dẫn ảnh trong zip theo thứ tự xuất hiện.
    images: Vec<String>,
    lines: Vec<Line>,
    cur: Line,
}

impl Walker<'_> {
    fn block(&mut self, node: &Node) {
        match node.name.as_str() {
            "w:p" => self.paragraph(node),
            "w:tbl" => {
                for tr in node.children_named("w:tr") {
                    for tc in tr.children_named("w:tc") {
                        for c in &tc.children {
                            self.block(c);
                        }
                    }
                }
            }
            "w:sdt" => {
                if let Some(content) = node.child("w:sdtContent") {
                    for c in &content.children {
                        self.block(c);
                    }
                }
            }
            "w:customXml" => {
                for c in &node.children {
                    self.block(c);
                }
            }
            _ => {}
        }
    }

    fn paragraph(&mut self, p: &Node) {
        self.flush();
        if let Some(label) = self.numbering.label(p.child("w:pPr")) {
            self.cur.push(Piece { text: format!("{label} "), ..Default::default() });
        }
        for c in &p.children {
            self.inline(c);
        }
        self.flush();
    }

    fn inline(&mut self, node: &Node) {
        match node.name.as_str() {
            "w:r" => {
                let style = run_style(node.child("w:rPr"));
                self.run_content(&node.children, style);
            }
            "w:hyperlink" | "w:smartTag" | "w:ins" | "w:sdt" | "w:sdtContent" | "w:fldSimple" | "w:customXml"
            | "w:moveTo" | "w:dir" | "w:bdo" => {
                for c in &node.children {
                    self.inline(c);
                }
            }
            "m:oMath" => self.math(node),
            "m:oMathPara" => {
                for m in node.children_named("m:oMath") {
                    self.math(m);
                }
            }
            "mc:AlternateContent" => {
                if let Some(choice) = node.child("mc:Choice").or_else(|| node.child("mc:Fallback")) {
                    for c in &choice.children {
                        self.inline(c);
                    }
                }
            }
            _ => {}
        }
    }

    fn run_content(&mut self, children: &[Node], style: RunStyle) {
        for c in children {
            match c.name.as_str() {
                "w:t" => {
                    let text = c.text_content();
                    let text = if style.sup || style.sub { script_text(&text, style.sup) } else { text };
                    self.text(&text, style);
                }
                "w:tab" | "w:ptab" => self.text("\t", style),
                "w:br" | "w:cr" => self.flush(),
                "w:noBreakHyphen" => self.text("-", style),
                "w:sym" => {
                    if let Some(ch) = symbol_char(c.attr("w:font").unwrap_or(""), c.attr("w:char").unwrap_or("")) {
                        self.text(&ch.to_string(), style);
                    }
                }
                "w:drawing" => {
                    if let Some(id) = c.find("a:blip").and_then(|b| b.attr("r:embed")) {
                        self.image(id);
                    }
                }
                "w:pict" => {
                    if let Some(id) = c.find("v:imagedata").and_then(|i| i.attr("r:id")) {
                        self.image(id);
                    }
                }
                "w:object" => {
                    if is_mathtype(c) {
                        self.atomic(MATHTYPE.to_string());
                    } else if let Some(id) = c.find("v:imagedata").and_then(|i| i.attr("r:id")) {
                        self.image(id);
                    }
                }
                "mc:AlternateContent" => {
                    if let Some(choice) = c.child("mc:Choice").or_else(|| c.child("mc:Fallback")) {
                        self.run_content(&choice.children, style);
                    }
                }
                _ => {}
            }
        }
    }

    fn text(&mut self, text: &str, style: RunStyle) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.cur.last_mut()
            && !last.atomic
            && last.bold == style.bold
            && last.italic == style.italic
            && last.marked == style.marked
        {
            last.text.push_str(text);
            return;
        }
        self.cur.push(Piece {
            text: text.to_string(),
            bold: style.bold,
            italic: style.italic,
            marked: style.marked,
            atomic: false,
        });
    }

    fn atomic(&mut self, text: String) {
        self.cur.push(Piece { text, atomic: true, ..Default::default() });
    }

    fn image(&mut self, rel_id: &str) {
        let Some(path) = self.rels.get(rel_id) else { return };
        let index = match self.images.iter().position(|p| p == path) {
            Some(i) => i,
            None if self.images.len() < MAX_IMAGES => {
                self.images.push(path.clone());
                self.images.len() - 1
            }
            None => return,
        };
        self.atomic(format!("![]({IMAGE_SCHEME}{index})"));
    }

    fn math(&mut self, node: &Node) {
        let tex = latex_of(node);
        let tex = tex.trim();
        if !tex.is_empty() {
            self.atomic(format!("${tex}$"));
        }
    }

    fn flush(&mut self) {
        let line = std::mem::take(&mut self.cur);
        if line.iter().any(|p| p.atomic || !p.text.trim().is_empty()) {
            self.lines.push(line);
        }
    }
}

// =====================================================================
// Công thức Word (OMML) → LaTeX
// =====================================================================

const MATH_FUNCTIONS: [&str; 26] = [
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh", "log", "ln", "lg",
    "exp", "max", "min", "sup", "inf", "det", "gcd", "lim", "tg", "cotg", "mod",
];

fn latex_of(node: &Node) -> String {
    node.children.iter().map(math_node).collect()
}

fn math_arg(node: &Node, name: &str) -> String {
    node.child(name).map(latex_of).unwrap_or_default()
}

/// Bọc `{…}` khi nhiều hơn một ký tự (cơ số của lũy thừa / chỉ số).
fn braced(s: &str) -> String {
    let s = s.trim();
    if s.chars().count() == 1 { s.to_string() } else { format!("{{{s}}}") }
}

fn hidden(pr: Option<&Node>, name: &str) -> bool {
    on(pr.and_then(|p| p.child(name)))
}

fn math_node(n: &Node) -> String {
    if n.is_text() {
        return String::new();
    }
    let name = n.name.as_str();
    match name {
        "m:r" => {
            let text: String = n.children_named("m:t").map(Node::text_content).collect();
            let normal = n.child("m:rPr").is_some_and(|p| p.child("m:nor").is_some());
            math_text(&text, normal)
        }
        "m:f" => {
            let (num, den) = (math_arg(n, "m:num"), math_arg(n, "m:den"));
            match n.child("m:fPr").and_then(|p| p.child("m:type")).and_then(Node::val) {
                Some("lin") | Some("skw") => format!("{}/{}", braced(&num), braced(&den)),
                Some("noBar") => format!("\\begin{{matrix}}{num} \\\\ {den}\\end{{matrix}}"),
                _ => format!("\\frac{{{num}}}{{{den}}}"),
            }
        }
        "m:sSup" => format!("{}^{{{}}}", braced(&math_arg(n, "m:e")), math_arg(n, "m:sup")),
        "m:sSub" => format!("{}_{{{}}}", braced(&math_arg(n, "m:e")), math_arg(n, "m:sub")),
        "m:sSubSup" => {
            format!("{}_{{{}}}^{{{}}}", braced(&math_arg(n, "m:e")), math_arg(n, "m:sub"), math_arg(n, "m:sup"))
        }
        "m:sPre" => format!("{{}}_{{{}}}^{{{}}}{}", math_arg(n, "m:sub"), math_arg(n, "m:sup"), math_arg(n, "m:e")),
        "m:rad" => {
            let e = math_arg(n, "m:e");
            let deg = math_arg(n, "m:deg");
            if hidden(n.child("m:radPr"), "m:degHide") || deg.trim().is_empty() {
                format!("\\sqrt{{{e}}}")
            } else {
                format!("\\sqrt[{}]{{{e}}}", deg.trim())
            }
        }
        "m:d" => delimiter(n),
        "m:nary" => nary(n),
        "m:func" => format!("{} {}", math_arg(n, "m:fName").trim_end(), math_arg(n, "m:e")),
        "m:limLow" => format!("{}_{{{}}}", math_arg(n, "m:e").trim_end(), math_arg(n, "m:lim")),
        "m:limUpp" => format!("{}^{{{}}}", math_arg(n, "m:e").trim_end(), math_arg(n, "m:lim")),
        "m:acc" => accent(n),
        "m:bar" => {
            let bottom = n.child("m:barPr").and_then(|p| p.child("m:pos")).and_then(Node::val) == Some("bot");
            format!("\\{}{{{}}}", if bottom { "underline" } else { "overline" }, math_arg(n, "m:e"))
        }
        "m:eqArr" => format!("\\begin{{aligned}}{}\\end{{aligned}}", rows(n)),
        "m:m" => {
            let rows: Vec<String> = n
                .children_named("m:mr")
                .map(|r| r.children_named("m:e").map(latex_of).collect::<Vec<_>>().join(" & "))
                .collect();
            format!("\\begin{{matrix}}{}\\end{{matrix}}", rows.join(" \\\\ "))
        }
        _ if name.ends_with("Pr") => String::new(),
        // m:e, m:box, m:borderBox, m:groupChr, m:phant, m:oMath…: lấy nội dung bên trong
        _ => latex_of(n),
    }
}

fn rows(eq_arr: &Node) -> String {
    eq_arr.children_named("m:e").map(latex_of).collect::<Vec<_>>().join(" \\\\ ")
}

fn fence(c: &str) -> String {
    match c {
        "" => ".".into(),
        "{" => "\\{".into(),
        "}" => "\\}".into(),
        "‖" => "\\Vert ".into(),
        "⟨" | "〈" => "\\langle ".into(),
        "⟩" | "〉" => "\\rangle ".into(),
        "⌊" => "\\lfloor ".into(),
        "⌋" => "\\rfloor ".into(),
        "⌈" => "\\lceil ".into(),
        "⌉" => "\\rceil ".into(),
        other => other.into(),
    }
}

fn delimiter(n: &Node) -> String {
    let pr = n.child("m:dPr");
    // Thẻ vắng mặt = ký tự mặc định; có thẻ mà không có val = không có ngoặc
    let chr = |name: &str, default: &str| match pr.and_then(|p| p.child(name)) {
        Some(c) => c.val().unwrap_or("").to_string(),
        None => default.to_string(),
    };
    let (beg, end, sep) = (chr("m:begChr", "("), chr("m:endChr", ")"), chr("m:sepChr", "|"));
    let parts: Vec<&Node> = n.children_named("m:e").collect();
    // Hệ phương trình: dấu { bên trái + mảng phương trình
    if beg == "{" && end.is_empty() && parts.len() == 1 {
        let inner: Vec<&Node> = parts[0].children.iter().filter(|c| !c.is_text() && !c.name.ends_with("Pr")).collect();
        if inner.len() == 1 && inner[0].name == "m:eqArr" {
            return format!("\\begin{{cases}}{}\\end{{cases}}", rows(inner[0]));
        }
    }
    let sep = if sep == "|" { " \\mid ".to_string() } else { sep };
    let body = parts.iter().map(|p| latex_of(p)).collect::<Vec<_>>().join(&sep);
    format!("\\left{} {} \\right{}", fence(&beg), body, fence(&end))
}

fn nary(n: &Node) -> String {
    let pr = n.child("m:naryPr");
    let chr = pr.and_then(|p| p.child("m:chr")).and_then(Node::val).unwrap_or("∫");
    let op = match chr {
        "∑" => "\\sum",
        "∏" => "\\prod",
        "∐" => "\\coprod",
        "∫" => "\\int",
        "∬" => "\\iint",
        "∭" => "\\iiint",
        "∮" => "\\oint",
        "⋃" => "\\bigcup",
        "⋂" => "\\bigcap",
        other => other,
    };
    let limit = |hide: &str, name: &str, mark: char| {
        let v = math_arg(n, name);
        if hidden(pr, hide) || v.trim().is_empty() { String::new() } else { format!("{mark}{{{}}}", v.trim()) }
    };
    format!("{op}{}{} {}", limit("m:subHide", "m:sub", '_'), limit("m:supHide", "m:sup", '^'), math_arg(n, "m:e"))
}

fn accent(n: &Node) -> String {
    let chr = n.child("m:accPr").and_then(|p| p.child("m:chr")).and_then(Node::val).unwrap_or("\u{0302}");
    let e = math_arg(n, "m:e");
    let wide = e.chars().filter(|c| c.is_alphanumeric()).count() > 1;
    let cmd = match chr {
        "\u{20D7}" | "\u{2192}" | "\u{20EF}" => {
            if wide {
                "overrightarrow"
            } else {
                "vec"
            }
        }
        "\u{20D6}" | "\u{2190}" => "overleftarrow",
        "\u{0303}" | "~" => {
            if wide {
                "widetilde"
            } else {
                "tilde"
            }
        }
        "\u{0304}" | "\u{0305}" | "\u{00AF}" | "\u{203E}" => "overline",
        "\u{0307}" => "dot",
        "\u{0308}" => "ddot",
        "\u{0311}" | "\u{2322}" | "\u{0306}" => "overarc",
        _ => {
            if wide {
                "widehat"
            } else {
                "hat"
            }
        }
    };
    format!("\\{cmd}{{{e}}}")
}

/// Chữ trong công thức: tên hàm → `\sin`, chữ tiếng Việt / chữ thường (m:nor) → `\text{…}`, ký tự đặc biệt LaTeX.
fn math_text(text: &str, normal: bool) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if MATH_FUNCTIONS.contains(&trimmed) {
        return format!("\\{trimmed} ");
    }
    let vietnamese = text
        .chars()
        .any(|c| c.is_alphabetic() && !c.is_ascii() && !('\u{0370}'..='\u{03FF}').contains(&c) && !"ℝℕℤℚℂ".contains(c));
    let wordy = trimmed.contains(' ') && trimmed.chars().filter(|c| c.is_alphabetic()).count() > 2;
    if normal || vietnamese || wordy {
        let clean: String = text.chars().filter(|c| !matches!(c, '{' | '}' | '\\' | '$')).collect();
        return format!("\\text{{{clean}}}");
    }
    let mut out = String::new();
    for c in text.chars() {
        match c {
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '%' => out.push_str("\\%"),
            '\\' => out.push_str("\\setminus "),
            '~' => out.push_str("\\sim "),
            '$' | '#' | '_' | '^' => {}
            // Ký tự vô hình Word chèn (áp dụng hàm, nhân ngầm, khoảng trắng không độ rộng)
            '\u{2061}' | '\u{2062}' | '\u{2063}' | '\u{2064}' | '\u{200B}' => {}
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out
}

// =====================================================================
// Các dòng → câu hỏi
// =====================================================================

fn line_chars(line: &Line) -> (Vec<char>, Vec<bool>, Vec<bool>) {
    let (mut chars, mut atomic, mut marked) = (Vec::new(), Vec::new(), Vec::new());
    for p in line {
        for c in p.text.chars() {
            chars.push(c);
            atomic.push(p.atomic);
            marked.push(p.marked);
        }
    }
    (chars, atomic, marked)
}

fn flush_md(out: &mut String, pending: Option<(bool, bool, String)>) {
    let Some((bold, italic, text)) = pending else { return };
    let core = text.trim();
    if core.is_empty() || (!bold && !italic) {
        out.push_str(&text);
        return;
    }
    let lead = &text[..text.len() - text.trim_start().len()];
    let trail = &text[text.trim_end().len()..];
    let mark = match (bold, italic) {
        (true, true) => "***",
        (true, false) => "**",
        _ => "*",
    };
    out.push_str(lead);
    out.push_str(mark);
    out.push_str(core);
    out.push_str(mark);
    out.push_str(trail);
}

/// Markdown của đoạn ký tự [from, to) trong dòng (giữ đậm / nghiêng, công thức và ảnh nguyên khối).
fn markdown(line: &Line, from: usize, to: usize) -> String {
    let mut out = String::new();
    let mut pending: Option<(bool, bool, String)> = None;
    let mut pos = 0;
    for p in line {
        let n = p.text.chars().count();
        let (start, end) = (pos, pos + n);
        pos = end;
        let (s, e) = (start.max(from), end.min(to));
        if s >= e {
            continue;
        }
        if p.atomic {
            flush_md(&mut out, pending.take());
            out.push_str(&p.text);
            continue;
        }
        let part: String = p.text.chars().skip(s - start).take(e - s).collect();
        match pending.as_mut() {
            Some((b, i, t)) if *b == p.bold && *i == p.italic => t.push_str(&part),
            _ => {
                flush_md(&mut out, pending.take());
                pending = Some((p.bold, p.italic, part));
            }
        }
    }
    flush_md(&mut out, pending);
    // Tab / nhiều dấu cách → một dấu cách
    let mut clean = String::with_capacity(out.len());
    for c in out.trim().chars() {
        let c = if c == '\t' || c == '\u{00A0}' { ' ' } else { c };
        if !(c == ' ' && clean.ends_with(' ')) {
            clean.push(c);
        }
    }
    clean
}

fn skip_ws(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

fn collect(chars: &[char]) -> String {
    chars.iter().collect()
}

/// "Câu 12 (NB). …" → (12, mức độ, vị trí bắt đầu nội dung).
fn question_header(chars: &[char]) -> Option<(usize, Option<String>, usize)> {
    let mut i = skip_ws(chars, 0);
    let start = i;
    while i < chars.len() && chars[i].is_alphabetic() {
        i += 1;
    }
    if !matches!(fold(&collect(&chars[start..i])).as_str(), "cau" | "bai" | "question") {
        return None;
    }
    i = skip_ws(chars, i);
    let digits = i;
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i == digits {
        return None;
    }
    let number = collect(&chars[digits..i]).parse().ok()?;
    if i < chars.len() && chars[i].is_alphanumeric() {
        return None; // "Câu 1a"
    }
    i = skip_ws(chars, i);
    let mut level = None;
    if i < chars.len() && matches!(chars[i], '(' | '[') {
        let close = if chars[i] == '(' { ')' } else { ']' };
        if let Some(j) = chars[i + 1..].iter().take(40).position(|&c| c == close) {
            let inner = collect(&chars[i + 1..i + 1 + j]).trim().to_string();
            // Chỉ nhận ngoặc là mức độ ("NB") hoặc điểm ("0,25 điểm")
            if parse_level(&inner).is_some() || fold(&inner).contains("diem") {
                if parse_level(&inner).is_some() {
                    level = Some(inner);
                }
                i = skip_ws(chars, i + 2 + j);
            }
        }
    }
    if i < chars.len() && matches!(chars[i], '.' | ':' | ')' | '-' | '–') {
        i += 1;
    }
    Some((number, level, skip_ws(chars, i)))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Choice,
    TrueFalse,
    Short,
    Essay,
}

/// Tiêu đề phần: "PHẦN I. …", "Phần 2: …". `Some(None)` = tiêu đề phần không rõ loại câu.
fn section(chars: &[char]) -> Option<Option<Section>> {
    let folded = fold(&collect(chars));
    let rest = folded.strip_prefix("phan ")?;
    let token: String = rest.chars().take_while(|c| c.is_alphanumeric()).collect();
    if token.is_empty() || !(token.chars().all(|c| c.is_ascii_digit()) || token.chars().all(|c| "ivx".contains(c))) {
        return None;
    }
    Some(if folded.contains("dung sai") || folded.contains("dung/sai") {
        Some(Section::TrueFalse)
    } else if folded.contains("tra loi ngan") || folded.contains("dien dap an") {
        Some(Section::Short)
    } else if folded.contains("tu luan") {
        Some(Section::Essay)
    } else if folded.contains("trac nghiem") || folded.contains("phuong an") || folded.contains("lua chon") {
        Some(Section::Choice)
    } else {
        None
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Label {
    Answer,
    Explanation,
    Level,
    Tags,
}

/// Các từ đầu dòng (bỏ dấu, chữ thường) kèm vị trí ngay sau từ.
fn leading_words(chars: &[char], n: usize) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut i = skip_ws(chars, 0);
    while out.len() < n && i < chars.len() && chars[i].is_alphanumeric() {
        let s = i;
        while i < chars.len() && chars[i].is_alphanumeric() {
            i += 1;
        }
        out.push((fold(&collect(&chars[s..i])), i));
        i = skip_ws(chars, i);
    }
    out
}

/// Dòng nhãn: "Đáp án: B", "Lời giải:", "Mức độ: VD", "Danh mục: Hàm số" → (nhãn, vị trí giá trị).
fn label(chars: &[char]) -> Option<(Label, usize)> {
    let words = leading_words(chars, 4);
    let w = |k: usize| words.get(k).map(|(s, _)| s.as_str());
    let end = |k: usize| words[k].1;
    let (label, at, needs_sep) = match (w(0), w(1), w(2), w(3)) {
        (Some("dap"), Some("an" | "so"), _, _) => (Label::Answer, end(1), false),
        (Some("da"), _, _, _) => (Label::Answer, end(0), true),
        (Some("loi"), Some("giai"), Some("chi"), Some("tiet")) => (Label::Explanation, end(3), false),
        (Some("loi"), Some("giai"), _, _) => (Label::Explanation, end(1), false),
        (Some("huong"), Some("dan"), Some("giai"), _) => (Label::Explanation, end(2), false),
        (Some("huong"), Some("dan"), _, _) => (Label::Explanation, end(1), true),
        (Some("hdg" | "giai"), _, _, _) => (Label::Explanation, end(0), true),
        (Some("muc"), Some("do"), _, _) => (Label::Level, end(1), true),
        (Some("danh"), Some("muc"), _, _) => (Label::Tags, end(1), true),
        _ => return None,
    };
    let mut i = skip_ws(chars, at);
    let has_sep = i < chars.len() && matches!(chars[i], ':' | '.');
    if has_sep {
        i += 1;
    } else if needs_sep && i < chars.len() {
        return None;
    }
    Some((label, skip_ws(chars, i)))
}

/// "Chọn B", "Chọn đáp án A, C." → chữ cái đáp án.
fn choose(chars: &[char]) -> Option<String> {
    let words = leading_words(chars, 3);
    if words.first().map(|(w, _)| w.as_str()) != Some("chon") {
        return None;
    }
    let at = if words.get(1).map(|(w, _)| w.as_str()) == Some("dap")
        && words.get(2).map(|(w, _)| w.as_str()) == Some("an")
    {
        words[2].1
    } else {
        words[0].1
    };
    let value = collect(&chars[at..]).trim().trim_end_matches('.').trim().to_string();
    (value.chars().count() <= 12 && parse_letters(&value).is_some_and(|v| !v.is_empty())).then_some(value)
}

/// Nhãn phương án trong dòng.
struct Mark {
    /// Vị trí bắt đầu nhãn (kể cả dấu *)
    at: usize,
    star: bool,
    /// Vị trí chữ cái
    letter_at: usize,
    /// Vị trí bắt đầu nội dung phương án
    content_at: usize,
}

/// Tìm các nhãn A. B. C. (hoặc a) b) c) khi `lower`) liên tiếp từ chữ thứ `next`; nhãn đầu phải ở đầu dòng
/// (trừ khi `anywhere` — dòng tiêu đề câu có kèm phương án, khi đó cần ít nhất 2 nhãn).
fn option_marks(chars: &[char], atomic: &[bool], next: usize, lower: bool, anywhere: bool) -> Vec<Mark> {
    let mut out: Vec<Mark> = Vec::new();
    let first = skip_ws(chars, 0);
    let mut i = 0;
    while i < chars.len() {
        let expected = next + out.len();
        if expected >= 6 {
            break;
        }
        let want = (if lower { b'a' } else { b'A' } + expected as u8) as char;
        let boundary = i == 0 || chars[i - 1].is_whitespace();
        if atomic[i] || !boundary {
            i += 1;
            continue;
        }
        let star = chars[i] == '*';
        let l = i + usize::from(star);
        let ok = l + 1 < chars.len()
            && chars[l] == want
            && !atomic[l]
            && (chars[l + 1] == ')' || (chars[l + 1] == '.' && (!lower || i == first)))
            && (l + 2 >= chars.len() || chars[l + 2].is_whitespace() || i == first);
        let ok = ok && (anywhere || !out.is_empty() || i == first);
        if ok {
            out.push(Mark { at: i, star, letter_at: l, content_at: skip_ws(chars, l + 2) });
            i = l + 2;
        } else {
            i += 1;
        }
    }
    if anywhere && out.len() < 2 {
        out.clear();
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Content,
    Options,
    Explanation,
}

struct DraftOption {
    text: String,
    marked: bool,
}

struct Draft {
    number: usize,
    section: Option<Section>,
    section_name: Option<String>,
    level_text: Option<String>,
    content: Vec<String>,
    options: Vec<DraftOption>,
    /// Các ý a) b) c) (đúng sai) thay vì A. B. C.
    statements: bool,
    answer: Option<String>,
    chosen: Option<String>,
    explanation: Vec<String>,
    tags: Vec<String>,
    mode: Mode,
}

impl Draft {
    fn new(number: usize, level: Option<String>, section: Option<Section>, section_name: Option<String>) -> Self {
        Self {
            number,
            section,
            section_name,
            level_text: level,
            content: Vec::new(),
            options: Vec::new(),
            statements: false,
            answer: None,
            chosen: None,
            explanation: Vec::new(),
            tags: Vec::new(),
            mode: Mode::Content,
        }
    }

    /// Thử đọc phương án trong dòng (từ ký tự `from`); trả `true` nếu có.
    fn take_options(&mut self, line: &Line, from: usize, anywhere: bool) -> bool {
        let (chars, atomic, marked) = line_chars(line);
        let (chars, atomic, marked) = (&chars[from..], &atomic[from..], &marked[from..]);
        let next = self.options.len();
        let mut lower = self.statements;
        let mut marks = option_marks(chars, atomic, next, lower, anywhere);
        if marks.is_empty() && next == 0 && !anywhere {
            marks = option_marks(chars, atomic, 0, true, false);
            lower = true;
        }
        if marks.is_empty() {
            return false;
        }
        if anywhere {
            // Phần trước nhãn đầu là nội dung câu hỏi
            let head = markdown(line, from, from + marks[0].at);
            if !head.is_empty() {
                self.content.push(head);
            }
        }
        self.statements = lower;
        for (k, m) in marks.iter().enumerate() {
            let end = marks.get(k + 1).map_or(chars.len(), |n| n.at);
            let body: Vec<usize> = (m.content_at..end).filter(|&j| !chars[j].is_whitespace()).collect();
            let all_marked = !body.is_empty() && body.iter().all(|&j| marked[j]);
            self.options.push(DraftOption {
                text: markdown(line, from + m.content_at, from + end),
                marked: m.star || marked[m.letter_at] || all_marked,
            });
        }
        self.mode = Mode::Options;
        true
    }

    fn feed(&mut self, line: &Line) {
        let (chars, _, _) = line_chars(line);
        if let Some((label, at)) = label(&chars) {
            let value = collect(&chars[at..]).trim().to_string();
            match label {
                Label::Answer => self.answer = Some(value),
                Label::Explanation => {
                    self.mode = Mode::Explanation;
                    let md = markdown(line, at, chars.len());
                    if !md.is_empty() {
                        self.explanation.push(md);
                    }
                }
                Label::Level => self.level_text = Some(value),
                Label::Tags => {
                    self.tags.extend(value.split(';').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()))
                }
            }
            return;
        }
        if self.mode != Mode::Content
            && let Some(letters) = choose(&chars)
        {
            self.chosen = Some(letters);
            if self.mode == Mode::Explanation {
                self.explanation.push(markdown(line, 0, chars.len()));
            }
            return;
        }
        if self.mode != Mode::Explanation && self.take_options(line, 0, false) {
            return;
        }
        let md = markdown(line, 0, chars.len());
        match self.mode {
            Mode::Content => self.content.push(md),
            Mode::Explanation => self.explanation.push(md),
            Mode::Options => {
                if let Some(last) = self.options.last_mut() {
                    if !last.text.is_empty() {
                        last.text.push('\n');
                    }
                    last.text.push_str(&md);
                }
            }
        }
    }

    fn finish(self, row: usize, tags: &[(String, Uuid)]) -> ImportItem {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let content = self.content.join("\n").trim().to_string();
        if content.is_empty() {
            errors.push("Thiếu nội dung câu hỏi.".to_string());
        }

        let has_options = !self.options.is_empty();
        let kind = match self.section {
            Some(Section::Essay) => {
                errors.push("Câu tự luận chưa nhập được (chỉ nhận trắc nghiệm, đúng sai, trả lời ngắn).".to_string());
                None
            }
            _ if self.statements => Some(QuestionKind::TrueFalse),
            Some(Section::TrueFalse) if has_options => Some(QuestionKind::TrueFalse),
            _ if has_options => Some(QuestionKind::Single),
            Some(Section::Choice | Section::TrueFalse) => {
                errors
                    .push("Không tìm thấy các phương án (dòng bắt đầu bằng A. B. C. D. hoặc a) b) c) d)).".to_string());
                None
            }
            _ => Some(QuestionKind::Short),
        };

        let marked: Vec<usize> = self.options.iter().enumerate().filter(|(_, o)| o.marked).map(|(i, _)| i).collect();
        let mut options: Vec<OptionInput> =
            self.options.iter().map(|o| OptionInput { id: None, content: o.text.clone(), correct: false }).collect();
        let mut accepted = Vec::new();
        let answer = self.answer.as_deref().map(str::trim).filter(|a| !a.is_empty());
        let mut kind = kind;
        match kind {
            Some(QuestionKind::Single | QuestionKind::Multiple) => {
                let from_label = answer.or(self.chosen.as_deref()).map(|a| (a, parse_letters(a)));
                let picked = match from_label {
                    Some((_, Some(idx))) if !idx.is_empty() => {
                        if !marked.is_empty() && marked != idx {
                            warnings.push(
                                "Đáp án ghi ở dòng \"Đáp án\" khác phương án được đánh dấu — dùng dòng \"Đáp án\"."
                                    .to_string(),
                            );
                        }
                        Some(idx)
                    }
                    Some((text, _)) => {
                        errors.push(format!("Đáp án \"{text}\" không hợp lệ (ghi chữ cái A–F)."));
                        None
                    }
                    None if !marked.is_empty() => Some(marked.clone()),
                    None => {
                        errors.push(
                            "Chưa có đáp án: ghi dòng \"Đáp án: B\" hoặc gạch chân / tô đỏ phương án đúng.".to_string(),
                        );
                        None
                    }
                };
                if let Some(idx) = picked {
                    for i in &idx {
                        match options.get_mut(*i) {
                            Some(o) => o.correct = true,
                            None => errors
                                .push(format!("Đáp án {} không có trong các phương án.", (b'A' + *i as u8) as char)),
                        }
                    }
                    if idx.len() > 1 {
                        kind = Some(QuestionKind::Multiple);
                    }
                }
            }
            Some(QuestionKind::TrueFalse) => match answer {
                Some(a) => match parse_truth(a) {
                    Some(marks) if marks.len() == options.len() => {
                        for (o, m) in options.iter_mut().zip(marks) {
                            o.correct = m;
                        }
                    }
                    _ => errors.push(format!(
                        "Câu đúng/sai: \"Đáp án\" phải có {} ký hiệu Đ/S tương ứng các ý.",
                        options.len()
                    )),
                },
                None if !marked.is_empty() => {
                    for i in marked {
                        options[i].correct = true;
                    }
                }
                None => errors.push(
                    "Chưa có đáp án: ghi dòng \"Đáp án: Đ S Đ S\" hoặc gạch chân / tô đỏ các ý đúng.".to_string(),
                ),
            },
            Some(QuestionKind::Short) => {
                accepted = answer
                    .unwrap_or_default()
                    .split(';')
                    .map(|a| a.trim().trim_end_matches('.').trim().to_string())
                    .filter(|a| !a.is_empty())
                    .collect();
                if accepted.is_empty() {
                    errors.push("Câu trả lời ngắn: thiếu dòng \"Đáp án: …\".".to_string());
                }
            }
            None => {}
        }

        let level = match self.level_text.as_deref() {
            Some(text) => {
                let l = parse_level(text);
                if l.is_none() {
                    errors.push(format!("Mức độ \"{text}\" không hợp lệ (NB, TH, VD, VDC)."));
                }
                l
            }
            None => {
                warnings.push("Chưa ghi mức độ, tạm đặt Nhận biết.".to_string());
                Some(QuestionLevel::NhanBiet)
            }
        };

        let mut tag_ids = Vec::new();
        for name in &self.tags {
            match tags.iter().find(|(t, _)| *t == fold(name)) {
                Some((_, id)) if !tag_ids.contains(id) => tag_ids.push(*id),
                Some(_) => {}
                None => warnings.push(format!("Danh mục \"{name}\" chưa có trong môn này — bỏ qua.")),
            }
        }

        let label = match &self.section_name {
            Some(s) => format!("{s} · Câu {}", self.number),
            None => format!("Câu {}", self.number),
        };
        ImportItem {
            row,
            label: Some(label),
            kind,
            level,
            content,
            options,
            accepted,
            explanation: self.explanation.join("\n").trim().to_string(),
            tag_ids,
            errors,
            warnings,
        }
    }
}

/// Tên ngắn của phần ("I", "II"…) để hiện cạnh số câu.
fn section_name(chars: &[char]) -> Option<String> {
    let words = leading_words(chars, 2);
    words.get(1).map(|(w, _)| w.to_uppercase())
}

fn lines_to_items(lines: &[Line], tags: &[(String, Uuid)]) -> Vec<ImportItem> {
    let mut items = Vec::new();
    let mut current: Option<Draft> = None;
    let mut sec: Option<Section> = None;
    let mut sec_name: Option<String> = None;
    for line in lines {
        let (chars, _, _) = line_chars(line);
        if let Some(s) = section(&chars) {
            if let Some(d) = current.take() {
                items.push(d.finish(items.len() + 1, tags));
            }
            sec = s;
            sec_name = section_name(&chars);
            continue;
        }
        if let Some((number, level, rest)) = question_header(&chars) {
            if let Some(d) = current.take() {
                items.push(d.finish(items.len() + 1, tags));
            }
            let mut d = Draft::new(number, level, sec, sec_name.clone());
            if !d.take_options(line, rest, true) {
                let md = markdown(line, rest, chars.len());
                if !md.is_empty() {
                    d.content.push(md);
                }
            }
            current = Some(d);
            continue;
        }
        // Trước câu đầu tiên (tên đề, hướng dẫn…): bỏ qua
        if let Some(d) = current.as_mut() {
            d.feed(line);
        }
    }
    if let Some(d) = current.take() {
        items.push(d.finish(items.len() + 1, tags));
    }
    items
}

/// Đọc file Word. `tags`: (tên danh mục đã bỏ dấu, id) của môn đang nhập.
pub fn parse_docx(bytes: &[u8], tags: &[(String, Uuid)]) -> AppResult<WordImport> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(invalid_file)?;
    let document = read_xml(&mut zip, "word/document.xml")?.ok_or_else(|| invalid_file("thiếu word/document.xml"))?;
    let rels = image_rels(read_xml(&mut zip, "word/_rels/document.xml.rels")?.as_ref());
    let numbering = Numbering::load(read_xml(&mut zip, "word/numbering.xml")?.as_ref());
    let body =
        document.child("w:document").and_then(|d| d.child("w:body")).ok_or_else(|| invalid_file("thiếu w:body"))?;

    let mut walker = Walker { rels: &rels, numbering, images: Vec::new(), lines: Vec::new(), cur: Vec::new() };
    for block in &body.children {
        walker.block(block);
    }
    walker.flush();

    let mut items = lines_to_items(&walker.lines, tags);
    if items.is_empty() {
        return Err(AppError::bad_request(
            "EMPTY_FILE",
            "Không tìm thấy câu hỏi nào. Mỗi câu cần bắt đầu bằng \"Câu 1.\", \"Câu 2.\"… (xem file mẫu).",
        ));
    }
    if items.len() > MAX_ROWS {
        return Err(AppError::bad_request(
            "TOO_MANY_ROWS",
            format!("Mỗi file tối đa {MAX_ROWS} câu. Hãy chia thành nhiều file."),
        ));
    }

    let mut images = Vec::new();
    for path in &walker.images {
        let name = path.rsplit('/').next().unwrap_or("anh").to_string();
        let content_type = image_type(path);
        let bytes = match content_type {
            Some(_) => read_entry(&mut zip, path, MAX_IMAGE_BYTES).ok().flatten(),
            None => None,
        };
        match bytes {
            Some(bytes) => images.push(WordImage { name, content_type, bytes }),
            None => images.push(WordImage { name, content_type: None, bytes: Vec::new() }),
        }
    }

    // Ảnh không hiện được trên web: thay bằng chữ + cảnh báo; công thức MathType: cảnh báo.
    for item in &mut items {
        let mut texts: Vec<&mut String> = vec![&mut item.content, &mut item.explanation];
        texts.extend(item.options.iter_mut().map(|o| &mut o.content));
        let mut bad_image = false;
        let mut mathtype = false;
        for text in texts {
            for (i, img) in images.iter().enumerate() {
                if img.content_type.is_none() || img.bytes.is_empty() {
                    let link = format!("![]({IMAGE_SCHEME}{i})");
                    if text.contains(&link) {
                        *text = text.replace(&link, UNSUPPORTED_IMAGE);
                        bad_image = true;
                    }
                }
            }
            mathtype |= text.contains(MATHTYPE);
        }
        if mathtype {
            item.warnings.push(
                "Có công thức MathType chưa đọc được (đang để chỗ trống). Trong Word chọn MathType → Convert Equations \
                 sang \"Word equation\" rồi nhập lại, hoặc gõ lại công thức dạng $…$."
                    .to_string(),
            );
        }
        if bad_image {
            item.warnings.push(
                "Có ảnh định dạng WMF/EMF (hoặc quá 5 MB) không hiện được trên web — hãy chèn lại ảnh PNG/JPG."
                    .to_string(),
            );
        }
    }
    Ok(WordImport { items, images })
}

// =====================================================================
// Ghi file .docx (file mẫu + dữ liệu test)
// =====================================================================

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Run chữ, `props` là nội dung `<w:rPr>` (đậm `<w:b/>`, gạch chân, màu…).
pub fn run(text: &str, props: &str) -> String {
    let rpr = if props.is_empty() { String::new() } else { format!("<w:rPr>{props}</w:rPr>") };
    format!(r#"<w:r>{rpr}<w:t xml:space="preserve">{}</w:t></w:r>"#, esc(text))
}

pub const BOLD: &str = "<w:b/>";
pub const UNDERLINE: &str = r#"<w:u w:val="single"/>"#;
pub const RED: &str = r#"<w:color w:val="FF0000"/>"#;
pub const TAB: &str = "<w:r><w:tab/></w:r>";

pub fn paragraph(runs: &[String]) -> String {
    format!("<w:p>{}</w:p>", runs.concat())
}

/// Công thức Word: `inner` là OMML bên trong `<m:oMath>`.
pub fn equation(inner: &str) -> String {
    format!("<m:oMath>{inner}</m:oMath>")
}

pub fn math_run(text: &str) -> String {
    format!("<m:r><m:t>{}</m:t></m:r>", esc(text))
}

/// Ghép file .docx tối thiểu từ phần thân `<w:body>`, ảnh `(tên file trong word/media, nội dung)` có
/// quan hệ `rIdImg1`, `rIdImg2`…
pub fn build_docx(body: &str, media: &[(&str, &[u8])]) -> AppResult<Vec<u8>> {
    let fail = |e: zip::result::ZipError| AppError::internal(e);
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture" xmlns:v="urn:schemas-microsoft-com:vml" xmlns:o="urn:schemas-microsoft-com:office:office"><w:body>{body}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1134" w:right="1134" w:bottom="1134" w:left="1134" w:header="709" w:footer="709" w:gutter="0"/></w:sectPr></w:body></w:document>"#
    );
    let rels: String = media
        .iter()
        .enumerate()
        .map(|(i, (name, _))| {
            format!(
                r#"<Relationship Id="rIdImg{}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/{}"/>"#,
                i + 1,
                esc(name)
            )
        })
        .collect();
    let files: Vec<(String, Vec<u8>)> = [
        (
            "[Content_Types].xml".to_string(),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Default Extension="jpeg" ContentType="image/jpeg"/><Default Extension="jpg" ContentType="image/jpeg"/><Default Extension="wmf" ContentType="image/x-wmf"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#
                .as_bytes()
                .to_vec(),
        ),
        (
            "_rels/.rels".to_string(),
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#
                .as_bytes()
                .to_vec(),
        ),
        ("word/document.xml".to_string(), document.into_bytes()),
        (
            "word/_rels/document.xml.rels".to_string(),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{rels}</Relationships>"#
            )
            .into_bytes(),
        ),
    ]
    .into_iter()
    .chain(media.iter().map(|(name, bytes)| (format!("word/media/{name}"), bytes.to_vec())))
    .collect();

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in files {
        zip.start_file(name, options).map_err(fail)?;
        zip.write_all(&bytes).map_err(AppError::internal)?;
    }
    Ok(zip.finish().map_err(fail)?.into_inner())
}

/// File mẫu tải về: hướng dẫn ngắn + 4 câu ví dụ (đủ 4 loại câu, công thức Word, gạch chân / tô đỏ đáp án).
pub fn template_docx() -> AppResult<Vec<u8>> {
    let sqrt16 = equation(&format!(
        r#"<m:rad><m:radPr><m:degHide m:val="1"/></m:radPr><m:deg/><m:e>{}</m:e></m:rad>"#,
        math_run("16")
    ));
    let half = equation(&format!(
        "<m:f><m:num>{}</m:num><m:den>{}</m:den></m:f>{}",
        math_run("1"),
        math_run("2"),
        math_run("x-1=0")
    ));
    let guide = [
        "Mỗi câu bắt đầu bằng \"Câu 1.\", \"Câu 2.\"… (gõ tay hoặc đánh số tự động). Mức độ ghi trong ngoặc: (NB), (TH), (VD), (VDC).",
        "Phương án: A. B. C. D. (có thể chung một dòng, cách nhau bằng phím Tab). Câu đúng sai: các ý a) b) c) d).",
        "Đáp án: gạch chân hoặc tô đỏ phương án / ý đúng, hoặc thêm dòng \"Đáp án: B\" (\"Đáp án: Đ S Đ S\", \"Đáp án: 0,5; 1/2\").",
        "Lời giải: dòng bắt đầu bằng \"Lời giải:\" — tới hết câu. Công thức: dùng Equation của Word (Insert → Equation) hoặc gõ $…$.",
        "Tiêu đề phần (PHẦN I. Trắc nghiệm…, PHẦN II. Đúng sai, PHẦN III. Trả lời ngắn) giúp nhận loại câu. Chữ trước câu 1 được bỏ qua.",
    ];
    let mut body = vec![paragraph(&[run("MẪU NHẬP CÂU HỎI TỪ WORD — Smart Learning Space", BOLD)])];
    body.extend(guide.iter().map(|g| paragraph(&[run(g, r#"<w:i/><w:color w:val="555555"/>"#)])));
    body.extend([
        paragraph(&[run("PHẦN I. Câu trắc nghiệm nhiều phương án lựa chọn", BOLD)]),
        paragraph(&[run("Câu 1 (NB). ", BOLD), run("Giá trị của ", ""), sqrt16, run(" là", "")]),
        paragraph(&[
            run("A. 2", ""),
            TAB.to_string(),
            run("B.", UNDERLINE),
            run(" 4", ""),
            TAB.to_string(),
            run("C. 8", ""),
            TAB.to_string(),
            run("D. 16", ""),
        ]),
        paragraph(&[run("Lời giải: ", BOLD), run("$\\sqrt{16} = 4$ vì $4^2 = 16$.", "")]),
        paragraph(&[run("Câu 2 (TH). ", BOLD), run("Những số nào là số nguyên tố?", "")]),
        paragraph(&[run("A. 2", "")]),
        paragraph(&[run("B. 4", "")]),
        paragraph(&[run("C. 7", "")]),
        paragraph(&[run("D. 9", "")]),
        paragraph(&[run("Đáp án: A, C", "")]),
        paragraph(&[run("PHẦN II. Câu trắc nghiệm đúng sai", BOLD)]),
        paragraph(&[run("Câu 3 (VD). ", BOLD), run("Cho hàm số $y = 2x + 1$.", "")]),
        paragraph(&[run("a) Đồ thị đi qua điểm $(0; 1)$.", RED)]),
        paragraph(&[run("b) Hàm số nghịch biến trên $\\mathbb{R}$.", "")]),
        paragraph(&[run("c) Hệ số góc bằng 2.", RED)]),
        paragraph(&[run("d) Đồ thị cắt trục hoành tại $x = 1$.", "")]),
        paragraph(&[run("PHẦN III. Câu trắc nghiệm trả lời ngắn", BOLD)]),
        paragraph(&[run("Câu 4 (VD). ", BOLD), run("Nghiệm của phương trình ", ""), half, run(" là", "")]),
        paragraph(&[run("Đáp án: 2", "")]),
    ]);
    build_docx(&body.concat(), &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n-anh-thu-";

    fn items(body: &[String]) -> Vec<ImportItem> {
        parse_docx(&build_docx(&body.concat(), &[]).unwrap(), &[]).unwrap().items
    }

    #[test]
    fn file_mau_doc_lai_duoc() {
        let parsed = parse_docx(&template_docx().unwrap(), &[]).unwrap();
        let items = parsed.items;
        assert_eq!(items.len(), 4);
        for item in &items {
            assert!(item.errors.is_empty(), "{:?}: {:?}", item.label, item.errors);
        }
        assert_eq!(items[0].label.as_deref(), Some("I · Câu 1"));
        assert_eq!(items[0].kind, Some(QuestionKind::Single));
        assert_eq!(items[0].level, Some(QuestionLevel::NhanBiet));
        assert_eq!(items[0].content, "Giá trị của $\\sqrt{16}$ là");
        assert_eq!(items[0].options.iter().map(|o| o.content.as_str()).collect::<Vec<_>>(), ["2", "4", "8", "16"]);
        assert!(items[0].options[1].correct && !items[0].options[0].correct);
        assert_eq!(items[0].explanation, "$\\sqrt{16} = 4$ vì $4^2 = 16$.");
        assert_eq!(items[1].kind, Some(QuestionKind::Multiple));
        assert_eq!(items[1].options.iter().map(|o| o.correct).collect::<Vec<_>>(), [true, false, true, false]);
        assert_eq!(items[2].kind, Some(QuestionKind::TrueFalse));
        assert_eq!(items[2].options[0].content, "Đồ thị đi qua điểm $(0; 1)$.");
        assert_eq!(items[2].options.iter().map(|o| o.correct).collect::<Vec<_>>(), [true, false, true, false]);
        assert_eq!(items[3].kind, Some(QuestionKind::Short));
        assert_eq!(items[3].content, "Nghiệm của phương trình $\\frac{1}{2}x-1=0$ là");
        assert_eq!(items[3].accepted, ["2"]);
        assert!(parsed.images.is_empty());
    }

    #[test]
    fn cong_thuc_word_sang_latex() {
        let tex = |inner: &str| {
            let xml = format!(r#"<m:oMath xmlns:m="m">{inner}</m:oMath>"#);
            latex_of(&parse_xml(&xml).unwrap().children[0]).trim().to_string()
        };
        assert_eq!(
            tex(&format!("<m:sSup><m:e>{}</m:e><m:sup>{}</m:sup></m:sSup>", math_run("x"), math_run("2"))),
            "x^{2}"
        );
        assert_eq!(
            tex(&format!("<m:rad><m:deg>{}</m:deg><m:e>{}</m:e></m:rad>", math_run("3"), math_run("x+1"))),
            "\\sqrt[3]{x+1}"
        );
        // Hệ phương trình
        assert_eq!(
            tex(&format!(
                r#"<m:d><m:dPr><m:begChr m:val="{{"/><m:endChr m:val=""/></m:dPr><m:e><m:eqArr><m:e>{}</m:e><m:e>{}</m:e></m:eqArr></m:e></m:d>"#,
                math_run("x+y=5"),
                math_run("2x−y=1")
            )),
            "\\begin{cases}x+y=5 \\\\ 2x−y=1\\end{cases}"
        );
        // Ngoặc tròn mặc định, hàm số, giới hạn, tổng, vector
        assert_eq!(tex(&format!("<m:d><m:e>{}</m:e></m:d>", math_run("a;b"))), "\\left( a;b \\right)");
        assert_eq!(
            tex(&format!("<m:func><m:fName>{}</m:fName><m:e>{}</m:e></m:func>", math_run("sin"), math_run("x"))),
            "\\sin x"
        );
        assert_eq!(
            tex(&format!(
                "<m:func><m:fName><m:limLow><m:e>{}</m:e><m:lim>{}</m:lim></m:limLow></m:fName><m:e>{}</m:e></m:func>",
                math_run("lim"),
                math_run("x→0"),
                math_run("f(x)")
            )),
            "\\lim_{x→0} f(x)"
        );
        assert_eq!(
            tex(&format!(
                r#"<m:nary><m:naryPr><m:chr m:val="∑"/></m:naryPr><m:sub>{}</m:sub><m:sup>{}</m:sup><m:e>{}</m:e></m:nary>"#,
                math_run("k=1"),
                math_run("n"),
                math_run("k")
            )),
            "\\sum_{k=1}^{n} k"
        );
        assert_eq!(
            tex(&format!(
                r#"<m:acc><m:accPr><m:chr m:val="&#x20D7;"/></m:accPr><m:e>{}</m:e></m:acc>"#,
                math_run("AB")
            )),
            "\\overrightarrow{AB}"
        );
        assert_eq!(tex(&format!(r#"<m:acc><m:e>{}</m:e></m:acc>"#, math_run("ABC"))), "\\widehat{ABC}");
        // Chữ tiếng Việt trong công thức, ký tự đặc biệt LaTeX
        assert_eq!(tex(&math_run("với x>0")), "\\text{với x>0}");
        assert_eq!(tex(&math_run("{1;2}")), "\\{1;2\\}");
    }

    #[test]
    fn danh_dau_dap_an_va_phuong_an_nhieu_kieu() {
        let items = items(&[
            paragraph(&[run("ĐỀ KIỂM TRA 15 PHÚT", BOLD)]),
            paragraph(&[run("Câu 1: Cho điểm A. Chọn khẳng định đúng", "")]),
            paragraph(&[run("A. Một", ""), TAB.to_string(), run("B. Hai", RED)]),
            paragraph(&[run("C. Ba", ""), TAB.to_string(), run("*D. Bốn", "")]),
            paragraph(&[run("Câu 2 [VDC]. 2 + 2 = ? A. 3 B. 4 C. 5", "")]),
            paragraph(&[run("Hướng dẫn giải", BOLD)]),
            paragraph(&[run("Ta có 2 + 2 = 4.", "")]),
            paragraph(&[run("A. sai vì 3 ≠ 4", "")]),
            paragraph(&[run("Chọn B.", "")]),
            paragraph(&[run("Câu 3. Phương trình vô nghiệm?", "")]),
            paragraph(&[run("Mức độ: TH", "")]),
            paragraph(&[
                run("Câu 4 (VD). Tính ", ""),
                run("x", ""),
                run("2", r#"<w:vertAlign w:val="superscript"/>"#),
                run(" khi x = 3", ""),
            ]),
            paragraph(&[run("Đáp án: 9", "")]),
        ]);
        assert_eq!(items.len(), 4);
        // Câu 1: "A." trong nội dung không bị nhận là phương án; tô đỏ B và dấu * ở D → nhiều đáp án
        assert_eq!(items[0].content, "Cho điểm A. Chọn khẳng định đúng");
        assert_eq!(items[0].kind, Some(QuestionKind::Multiple));
        assert_eq!(items[0].options.iter().map(|o| o.correct).collect::<Vec<_>>(), [false, true, false, true]);
        assert_eq!(items[0].options[3].content, "Bốn");
        // Câu 2: phương án chung dòng tiêu đề; "Chọn B" trong lời giải; dòng "A. sai…" thuộc lời giải
        assert_eq!(items[1].level, Some(QuestionLevel::VanDungCao));
        assert_eq!(items[1].content, "2 + 2 = ?");
        assert_eq!(items[1].options.len(), 3);
        assert!(items[1].options[1].correct);
        assert_eq!(items[1].explanation, "Ta có 2 + 2 = 4.\nA. sai vì 3 ≠ 4\nChọn B.");
        // Câu 3: không có phương án, không có đáp án → lỗi
        assert_eq!(items[2].kind, Some(QuestionKind::Short));
        assert_eq!(items[2].level, Some(QuestionLevel::ThongHieu));
        assert!(items[2].errors.iter().any(|e| e.contains("thiếu dòng \"Đáp án")));
        // Câu 4: chỉ số trên gõ bằng định dạng chữ
        assert_eq!(items[3].content, "Tính x² khi x = 3");
        assert_eq!(items[3].accepted, ["9"]);
    }

    #[test]
    fn danh_so_tu_dong_anh_mathtype() {
        let numbering = r#"<?xml version="1.0" encoding="UTF-8"?>
<w:numbering xmlns:w="w"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="Câu %1."/></w:lvl><w:lvl w:ilvl="1"><w:start w:val="1"/><w:numFmt w:val="upperLetter"/><w:lvlText w:val="%2."/></w:lvl></w:abstractNum><w:num w:numId="5"><w:abstractNumId w:val="0"/></w:num></w:numbering>"#;
        let num = |lvl: u8, runs: &[String]| {
            format!(
                r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{lvl}"/><w:numId w:val="5"/></w:numPr></w:pPr>{}</w:p>"#,
                runs.concat()
            )
        };
        let image = r#"<w:r><w:drawing><wp:inline><a:graphic><a:graphicData><pic:pic><pic:blipFill><a:blip r:embed="rIdImg1"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>"#;
        let wmf = r#"<w:r><w:pict><v:shape><v:imagedata r:id="rIdImg2"/></v:shape></w:pict></w:r>"#;
        let mathtype = r#"<w:r><w:object><v:shape><v:imagedata r:id="rIdImg2"/></v:shape><o:OLEObject Type="Embed" ProgID="Equation.DSMT4"/></w:object></w:r>"#;
        let body = [
            num(0, &[run("Hình vẽ bên là đồ thị hàm số nào?", "")]),
            paragraph(&[image.to_string()]),
            num(1, &[run("y = x", "")]),
            num(1, &[run("y = 2x", UNDERLINE)]),
            num(0, &[run("Giá trị ", ""), mathtype.to_string(), run(" bằng", "")]),
            num(1, &[run("1", ""), wmf.to_string()]),
            num(1, &[run("2", RED)]),
        ]
        .concat();
        let mut bytes = build_docx(&body, &[("image1.png", PNG), ("image2.wmf", b"\xD7\xCD\xC6\x9A")]).unwrap();
        // Chèn numbering.xml vào file zip
        {
            let mut zip = ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
            let mut out = ZipWriter::new(Cursor::new(Vec::new()));
            for i in 0..zip.len() {
                let mut f = zip.by_index(i).unwrap();
                let name = f.name().unwrap().to_string();
                let mut data = Vec::new();
                f.read_to_end(&mut data).unwrap();
                out.start_file(name, SimpleFileOptions::default()).unwrap();
                out.write_all(&data).unwrap();
            }
            out.start_file("word/numbering.xml", SimpleFileOptions::default()).unwrap();
            out.write_all(numbering.as_bytes()).unwrap();
            bytes = out.finish().unwrap().into_inner();
        }
        let parsed = parse_docx(&bytes, &[]).unwrap();
        assert_eq!(parsed.items.len(), 2);
        let q1 = &parsed.items[0];
        assert_eq!(q1.label.as_deref(), Some("Câu 1"));
        assert_eq!(q1.content, "Hình vẽ bên là đồ thị hàm số nào?\n![](docx-image:0)");
        assert_eq!(q1.options.iter().map(|o| o.correct).collect::<Vec<_>>(), [false, true]);
        assert_eq!(parsed.images[0].content_type, Some("image/png"));
        assert_eq!(parsed.images[0].bytes, PNG);
        let q2 = &parsed.items[1];
        assert_eq!(q2.label.as_deref(), Some("Câu 2"));
        assert_eq!(q2.content, format!("Giá trị {MATHTYPE} bằng"));
        assert_eq!(q2.options[0].content, format!("1{UNSUPPORTED_IMAGE}"));
        assert!(q2.warnings.iter().any(|w| w.contains("MathType")));
        assert!(q2.warnings.iter().any(|w| w.contains("WMF")));
        assert!(q2.options[1].correct);
    }

    #[test]
    fn file_khong_phai_word() {
        assert!(parse_docx(b"khong phai zip", &[]).is_err());
        let empty = build_docx(&paragraph(&[run("Không có câu nào", "")]), &[]).unwrap();
        let err = parse_docx(&empty, &[]).err().unwrap();
        assert_eq!(err.code, "EMPTY_FILE");
        assert_eq!(format_number(28, "upperLetter"), "BB");
        assert_eq!(format_number(14, "upperRoman"), "XIV");
    }
}

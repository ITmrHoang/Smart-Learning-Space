//! Import câu hỏi từ Excel theo file mẫu, và tạo file mẫu.
//!
//! Cột (dòng 1 là tiêu đề): Loại câu · Mức độ · Nội dung · Phương án A…F · Đáp án đúng · Lời giải · Danh mục.
//! - Một / nhiều đáp án: "Đáp án đúng" ghi chữ cái (`B`, `A, C`).
//! - Đúng/sai: các ý a, b, c, d ghi ở cột Phương án A–D; "Đáp án đúng" ghi `Đ S Đ S` (hoặc `ĐSĐS`, `T F T F`).
//! - Trả lời ngắn: "Đáp án đúng" ghi các đáp án chấp nhận, cách nhau dấu `;` (`0,5; 1/2`).
//! - Danh mục: tên danh mục của môn, cách nhau dấu `;`.
//! - Công thức gõ dạng LaTeX `$...$`.

use std::io::Cursor;

use calamine::{Data, Reader, Xlsx};
use rust_xlsxwriter::{Format, Workbook};
use uuid::Uuid;

use super::{dto::ImportItem, dto::OptionInput, model::QuestionKind};
use crate::{
    error::{AppError, AppResult},
    modules::catalog::model::QuestionLevel,
    slug::{fold, slugify},
};

pub const MAX_ROWS: usize = 500;
const OPTION_COLS: usize = 6; // Phương án A–F
const COL_KIND: usize = 0;
const COL_LEVEL: usize = 1;
const COL_CONTENT: usize = 2;
const COL_FIRST_OPTION: usize = 3;
const COL_ANSWER: usize = COL_FIRST_OPTION + OPTION_COLS;
const COL_EXPLANATION: usize = COL_ANSWER + 1;
const COL_TAGS: usize = COL_EXPLANATION + 1;

const HEADERS: [&str; 12] = [
    "Loại câu",
    "Mức độ",
    "Nội dung",
    "Phương án A",
    "Phương án B",
    "Phương án C",
    "Phương án D",
    "Phương án E",
    "Phương án F",
    "Đáp án đúng",
    "Lời giải",
    "Danh mục",
];

fn invalid_file(detail: impl std::fmt::Display) -> AppError {
    tracing::info!(error = %detail, "file Excel không đọc được");
    AppError::bad_request("INVALID_FILE", "Không đọc được file. Hãy dùng file .xlsx theo mẫu.")
}

fn cell_str(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(s)) => s.trim().to_string(),
        Some(Data::Float(f)) => {
            if f.fract() == 0.0 && f.abs() < 1e15 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Some(Data::Int(i)) => i.to_string(),
        Some(Data::Bool(b)) => if *b { "Đúng" } else { "Sai" }.to_string(),
        Some(other) => other.to_string().trim().to_string(),
        None => String::new(),
    }
}

pub fn parse_kind(value: &str) -> Option<QuestionKind> {
    match slugify(value).as_str() {
        "mot-dap-an" | "1-dap-an" | "single" | "trac-nghiem" | "nhieu-lua-chon" => Some(QuestionKind::Single),
        "nhieu-dap-an" | "multiple" | "chon-nhieu" => Some(QuestionKind::Multiple),
        "dung-sai" | "true-false" | "dung-sai-4-y" | "d-s" => Some(QuestionKind::TrueFalse),
        "tra-loi-ngan" | "short" | "dien-dap-an" | "tu-dien" => Some(QuestionKind::Short),
        _ => None,
    }
}

pub fn parse_level(value: &str) -> Option<QuestionLevel> {
    match slugify(value).as_str() {
        "nhan-biet" | "nb" | "nhan_biet" => Some(QuestionLevel::NhanBiet),
        "thong-hieu" | "th" => Some(QuestionLevel::ThongHieu),
        "van-dung" | "vd" => Some(QuestionLevel::VanDung),
        "van-dung-cao" | "vdc" => Some(QuestionLevel::VanDungCao),
        _ => None,
    }
}

/// `B`, `A, C`, `ac` → [1], [0, 2]. Ký tự ngoài A–F → `None`.
pub fn parse_letters(value: &str) -> Option<Vec<usize>> {
    let mut out = Vec::new();
    for c in value.chars().filter(|c| c.is_alphanumeric()) {
        let idx = "abcdef".find(c.to_ascii_lowercase())?;
        if !out.contains(&idx) {
            out.push(idx);
        }
    }
    Some(out)
}

/// `Đ S Đ S`, `ĐSĐS`, `Đúng; Sai`, `T F`, `1 0` → [true, false, …].
pub fn parse_truth(value: &str) -> Option<Vec<bool>> {
    let folded = fold(value);
    let tokens: Vec<&str> = folded.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
    let words = |t: &str| match t {
        "dung" | "d" | "t" | "true" | "1" | "x" => Some(true),
        "sai" | "s" | "f" | "false" | "0" => Some(false),
        _ => None,
    };
    let mut out = Vec::new();
    for t in tokens {
        if let Some(v) = words(t) {
            out.push(v);
        } else {
            // Viết liền: "dsds"
            for c in t.chars() {
                out.push(words(&c.to_string())?);
            }
        }
    }
    Some(out)
}

/// Đọc file Excel. `tags`: (tên danh mục đã bỏ dấu, id) của môn đang import.
pub fn parse_xlsx(bytes: &[u8], tags: &[(String, Uuid)]) -> AppResult<Vec<ImportItem>> {
    let mut wb = Xlsx::new(Cursor::new(bytes)).map_err(invalid_file)?;
    let range = wb.worksheet_range_at(0).ok_or_else(|| invalid_file("không có sheet"))?.map_err(invalid_file)?;
    let mut items = Vec::new();
    for (i, row) in range.rows().enumerate().skip(1) {
        let get = |col: usize| cell_str(row.get(col));
        if (0..=COL_TAGS).all(|c| get(c).is_empty()) {
            continue;
        }
        if items.len() >= MAX_ROWS {
            return Err(AppError::bad_request(
                "TOO_MANY_ROWS",
                format!("Mỗi file tối đa {MAX_ROWS} câu. Hãy chia thành nhiều file."),
            ));
        }
        items.push(parse_row(i + 1, &get, tags));
    }
    if items.is_empty() {
        return Err(AppError::bad_request("EMPTY_FILE", "File không có câu hỏi nào (dòng 1 là tiêu đề)."));
    }
    Ok(items)
}

fn parse_row(row: usize, get: &dyn Fn(usize) -> String, tags: &[(String, Uuid)]) -> ImportItem {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let option_texts: Vec<String> = (0..OPTION_COLS).map(|k| get(COL_FIRST_OPTION + k)).collect::<Vec<_>>();
    let last = option_texts.iter().rposition(|t| !t.is_empty());
    let option_texts: Vec<String> = match last {
        Some(l) => option_texts[..=l].to_vec(),
        None => Vec::new(),
    };

    let kind_text = get(COL_KIND);
    let kind = if kind_text.is_empty() {
        Some(if option_texts.is_empty() { QuestionKind::Short } else { QuestionKind::Single })
    } else {
        let k = parse_kind(&kind_text);
        if k.is_none() {
            errors.push(format!(
                "Loại câu \"{kind_text}\" không hợp lệ (dùng: Một đáp án, Nhiều đáp án, Đúng sai, Trả lời ngắn)."
            ));
        }
        k
    };

    let level_text = get(COL_LEVEL);
    let level = if level_text.is_empty() {
        warnings.push("Chưa ghi mức độ, tạm đặt Nhận biết.".to_string());
        Some(QuestionLevel::NhanBiet)
    } else {
        let l = parse_level(&level_text);
        if l.is_none() {
            errors.push(format!("Mức độ \"{level_text}\" không hợp lệ (NB, TH, VD, VDC)."));
        }
        l
    };

    let content = get(COL_CONTENT);
    if content.is_empty() {
        errors.push("Thiếu nội dung câu hỏi.".to_string());
    }
    if option_texts.iter().any(String::is_empty) {
        errors.push("Có phương án bị bỏ trống ở giữa.".to_string());
    }

    let answer = get(COL_ANSWER);
    let mut options: Vec<OptionInput> =
        option_texts.iter().map(|t| OptionInput { id: None, content: t.clone(), correct: false }).collect();
    let mut accepted = Vec::new();
    match kind {
        Some(QuestionKind::Single | QuestionKind::Multiple) => match parse_letters(&answer) {
            Some(idx) if !idx.is_empty() => {
                for i in idx {
                    match options.get_mut(i) {
                        Some(o) => o.correct = true,
                        None => errors.push(format!("Đáp án đúng \"{answer}\" không có trong các phương án.")),
                    }
                }
            }
            _ => errors.push("Thiếu hoặc sai \"Đáp án đúng\" (ghi chữ cái A–F).".to_string()),
        },
        Some(QuestionKind::TrueFalse) => match parse_truth(&answer) {
            Some(marks) if marks.len() == options.len() && !marks.is_empty() => {
                for (o, m) in options.iter_mut().zip(marks) {
                    o.correct = m;
                }
            }
            _ => errors
                .push(format!("Câu đúng/sai: \"Đáp án đúng\" phải có {} ký hiệu Đ/S tương ứng các ý.", options.len())),
        },
        Some(QuestionKind::Short) => {
            accepted = answer.split(';').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect();
            if accepted.is_empty() {
                errors.push("Thiếu đáp án của câu trả lời ngắn.".to_string());
            }
            if !options.is_empty() {
                warnings.push("Câu trả lời ngắn không dùng các cột phương án — đã bỏ qua.".to_string());
                options.clear();
            }
        }
        None => {}
    }

    let mut tag_ids = Vec::new();
    for name in get(COL_TAGS).split(';').map(str::trim).filter(|n| !n.is_empty()) {
        match tags.iter().find(|(t, _)| *t == fold(name)) {
            Some((_, id)) if !tag_ids.contains(id) => tag_ids.push(*id),
            Some(_) => {}
            None => warnings.push(format!("Danh mục \"{name}\" chưa có trong môn này — bỏ qua.")),
        }
    }

    ImportItem {
        row,
        kind,
        level,
        content,
        options,
        accepted,
        explanation: get(COL_EXPLANATION),
        tag_ids,
        errors,
        warnings,
    }
}

/// File mẫu: sheet "Câu hỏi" (tiêu đề + 4 ví dụ) và sheet "Hướng dẫn".
pub fn template_xlsx() -> AppResult<Vec<u8>> {
    let fail = |e: rust_xlsxwriter::XlsxError| AppError::internal(e);
    let mut wb = Workbook::new();
    let bold = Format::new().set_bold().set_background_color("#E8F0FE");
    let wrap = Format::new().set_text_wrap();

    let ws = wb.add_worksheet();
    ws.set_name("Câu hỏi").map_err(fail)?;
    for (col, h) in HEADERS.iter().enumerate() {
        ws.write_string_with_format(0, col as u16, *h, &bold).map_err(fail)?;
        ws.set_column_width(col as u16, if col == COL_CONTENT || col == COL_EXPLANATION { 45 } else { 16 })
            .map_err(fail)?;
    }
    let samples: [[&str; 12]; 4] = [
        [
            "Một đáp án",
            "NB",
            "Giá trị của $\\sqrt{16}$ là",
            "2",
            "4",
            "8",
            "16",
            "",
            "",
            "B",
            "$\\sqrt{16} = 4$ vì $4^2 = 16$.",
            "",
        ],
        ["Nhiều đáp án", "TH", "Những số nào là số nguyên tố?", "2", "4", "7", "9", "", "", "A, C", "", ""],
        [
            "Đúng sai",
            "VD",
            "Cho hàm số $y = 2x + 1$.",
            "a) Đồ thị đi qua điểm $(0; 1)$.",
            "b) Hàm số nghịch biến trên $\\mathbb{R}$.",
            "c) Hệ số góc bằng 2.",
            "d) Đồ thị cắt trục hoành tại $x = 1$.",
            "",
            "",
            "Đ S Đ S",
            "",
            "",
        ],
        ["Trả lời ngắn", "VD", "Nghiệm của phương trình $2x - 1 = 0$ là", "", "", "", "", "", "", "0,5; 1/2", "", ""],
    ];
    for (r, row) in samples.iter().enumerate() {
        for (c, value) in row.iter().enumerate() {
            if !value.is_empty() {
                ws.write_string_with_format(r as u32 + 1, c as u16, *value, &wrap).map_err(fail)?;
            }
        }
    }

    let guide = wb.add_worksheet();
    guide.set_name("Hướng dẫn").map_err(fail)?;
    guide.set_column_width(0, 110).map_err(fail)?;
    let lines = [
        "Mỗi dòng ở sheet \"Câu hỏi\" là một câu. Dòng 1 là tiêu đề, giữ nguyên.",
        "Loại câu: Một đáp án · Nhiều đáp án · Đúng sai · Trả lời ngắn.",
        "Mức độ: Nhận biết (NB) · Thông hiểu (TH) · Vận dụng (VD) · Vận dụng cao (VDC).",
        "Phương án A–F: tối đa 6 phương án. Câu đúng/sai: ghi các ý a), b), c), d) vào cột Phương án A–D.",
        "Đáp án đúng: Một / nhiều đáp án ghi chữ cái (B hoặc A, C). Đúng sai ghi Đ/S theo thứ tự các ý (Đ S Đ S).",
        "Trả lời ngắn ghi các đáp án chấp nhận, cách nhau dấu chấm phẩy (0,5; 1/2). Số 0,5 và 0.5 được coi là như nhau.",
        "Công thức gõ dạng LaTeX đặt giữa hai dấu $, ví dụ $\\frac{1}{2}$, $x^2 + y^2$, $\\sqrt{3}$.",
        "Danh mục: tên danh mục của môn (đã có trên hệ thống), nhiều danh mục cách nhau dấu chấm phẩy.",
        "Tối đa 500 câu mỗi file. Sau khi tải lên, hệ thống hiện bảng xem trước để kiểm tra trước khi lưu.",
    ];
    for (i, line) in lines.iter().enumerate() {
        guide.write_string(i as u32, 0, *line).map_err(fail)?;
    }
    wb.save_to_buffer().map_err(fail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doc_loai_cau_va_muc_do() {
        assert_eq!(parse_kind("Một đáp án"), Some(QuestionKind::Single));
        assert_eq!(parse_kind("ĐÚNG SAI"), Some(QuestionKind::TrueFalse));
        assert_eq!(parse_kind("Trả lời ngắn"), Some(QuestionKind::Short));
        assert_eq!(parse_kind("tự luận"), None);
        assert_eq!(parse_level("VDC"), Some(QuestionLevel::VanDungCao));
        assert_eq!(parse_level("Thông hiểu"), Some(QuestionLevel::ThongHieu));
    }

    #[test]
    fn doc_dap_an() {
        assert_eq!(parse_letters("B"), Some(vec![1]));
        assert_eq!(parse_letters("A, c"), Some(vec![0, 2]));
        assert_eq!(parse_letters("G"), None);
        assert_eq!(parse_truth("Đ S Đ S"), Some(vec![true, false, true, false]));
        assert_eq!(parse_truth("ĐSĐS"), Some(vec![true, false, true, false]));
        assert_eq!(parse_truth("Đúng; Sai; Sai; Đúng"), Some(vec![true, false, false, true]));
        assert_eq!(parse_truth("T F x"), Some(vec![true, false, true]));
        assert_eq!(parse_truth("Đ ? S"), Some(vec![true, false]));
        assert_eq!(parse_truth("abc"), None);
    }

    #[test]
    fn file_mau_doc_lai_duoc() {
        let bytes = template_xlsx().unwrap();
        let items = parse_xlsx(&bytes, &[]).unwrap();
        assert_eq!(items.len(), 4);
        for item in &items {
            assert!(item.errors.is_empty(), "dòng {}: {:?}", item.row, item.errors);
        }
        assert_eq!(items[0].kind, Some(QuestionKind::Single));
        assert!(items[0].options[1].correct);
        assert_eq!(items[2].options.iter().map(|o| o.correct).collect::<Vec<_>>(), vec![true, false, true, false]);
        assert_eq!(items[3].accepted, vec!["0,5", "1/2"]);
    }
}

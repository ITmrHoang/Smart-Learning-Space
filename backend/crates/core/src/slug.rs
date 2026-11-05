//! Tạo slug từ tên tiếng Việt: "Lớp 10 Chuyên" → "lop-10-chuyen".

/// Chuỗi để tìm kiếm: bỏ dấu, chữ thường, gộp khoảng trắng ("Hàm  SỐ" → "ham so").
pub fn fold(input: &str) -> String {
    deunicode::deunicode(input).to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn slugify(input: &str) -> String {
    // deunicode bỏ dấu tiếng Việt ("ớ" → "o", "đ" → "d").
    let ascii = deunicode::deunicode(input).to_lowercase();
    let mut out = String::with_capacity(ascii.len());
    let mut dash = false;
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            if dash && !out.is_empty() {
                out.push('-');
            }
            out.push(c);
            dash = false;
        } else {
            dash = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn bo_dau_tieng_viet() {
        assert_eq!(slugify("Lớp 10 Chuyên"), "lop-10-chuyen");
        assert_eq!(slugify("  Đại số & Giải tích!! "), "dai-so-giai-tich");
        assert_eq!(slugify("Ôn thi IELTS"), "on-thi-ielts");
        assert_eq!(slugify("Ngữ văn"), "ngu-van");
        assert_eq!(slugify("!!!"), "");
    }
}

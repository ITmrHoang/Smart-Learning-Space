//! File lịch `.ics` (RFC 5545) gửi kèm email mời: Google Calendar, Outlook, lịch điện thoại tự thêm buổi học.
//! Dời / hủy buổi gửi lại cùng `UID`, tăng `SEQUENCE` → lịch của học sinh tự cập nhật / tự xóa, không bị nhân đôi.

use chrono::{DateTime, Utc};

pub const UID_DOMAIN: &str = "smart-learning-space";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    /// Mời / cập nhật.
    Request,
    /// Hủy.
    Cancel,
}

#[derive(Debug, Clone)]
pub struct IcsEvent {
    pub uid: String,
    pub sequence: i32,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub summary: String,
    pub description: String,
    pub url: Option<String>,
    pub organizer: String,
}

/// Thoát ký tự đặc biệt của giá trị TEXT.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace("\r\n", "\\n").replace('\n', "\\n")
}

/// Gấp dòng dài hơn 75 byte (không cắt giữa ký tự UTF-8), dòng nối bắt đầu bằng một dấu cách.
fn fold(line: &str) -> String {
    let mut out = String::new();
    let mut len = 0;
    for c in line.chars() {
        let n = c.len_utf8();
        if len + n > 75 {
            out.push_str("\r\n ");
            len = 1;
        }
        out.push(c);
        len += n;
    }
    out
}

fn stamp(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

/// Một file lịch chứa nhiều buổi (lịch lặp gửi tất cả buổi trong một file).
pub fn calendar(method: Method, events: &[IcsEvent], now: DateTime<Utc>) -> String {
    let mut lines: Vec<String> = vec![
        "BEGIN:VCALENDAR".into(),
        "VERSION:2.0".into(),
        "PRODID:-//Smart Learning Space//Lich hoc//VI".into(),
        "CALSCALE:GREGORIAN".into(),
        format!("METHOD:{}", if method == Method::Cancel { "CANCEL" } else { "REQUEST" }),
    ];
    for e in events {
        lines.push("BEGIN:VEVENT".into());
        lines.push(format!("UID:{}", e.uid));
        lines.push(format!("SEQUENCE:{}", e.sequence));
        lines.push(format!("DTSTAMP:{}", stamp(now)));
        lines.push(format!("DTSTART:{}", stamp(e.start)));
        lines.push(format!("DTEND:{}", stamp(e.end)));
        lines.push(format!("SUMMARY:{}", escape(&e.summary)));
        let mut desc = e.description.clone();
        if let Some(url) = &e.url {
            if !desc.is_empty() {
                desc.push_str("\n\n");
            }
            desc.push_str(&format!("Vào học: {url}"));
        }
        if !desc.is_empty() {
            lines.push(format!("DESCRIPTION:{}", escape(&desc)));
        }
        if let Some(url) = &e.url {
            lines.push(format!("URL:{url}"));
            lines.push(format!("LOCATION:{}", escape(url)));
        }
        lines.push(format!("ORGANIZER;CN={}:mailto:no-reply@{UID_DOMAIN}.local", escape(&e.organizer)));
        lines.push(format!("STATUS:{}", if method == Method::Cancel { "CANCELLED" } else { "CONFIRMED" }));
        if method == Method::Request {
            lines.push("BEGIN:VALARM".into());
            lines.push("ACTION:DISPLAY".into());
            lines.push(format!("DESCRIPTION:{}", escape(&e.summary)));
            lines.push("TRIGGER:-PT15M".into());
            lines.push("END:VALARM".into());
        }
        lines.push("END:VEVENT".into());
    }
    lines.push("END:VCALENDAR".into());
    let mut out = lines.iter().map(|l| fold(l)).collect::<Vec<_>>().join("\r\n");
    out.push_str("\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event() -> IcsEvent {
        IcsEvent {
            uid: "abc@smart-learning-space".into(),
            sequence: 2,
            start: "2026-11-10T12:30:00Z".parse().unwrap(),
            end: "2026-11-10T14:00:00Z".parse().unwrap(),
            summary: "Toán 9; buổi 1, căn bậc hai".into(),
            description: "Ôn tập\nchương 1".into(),
            url: Some("https://meet.google.com/abc-defg-hij".into()),
            organizer: "Cô Lan".into(),
        }
    }

    #[test]
    fn tao_file_moi_va_huy() {
        let now = "2026-11-08T01:02:03Z".parse().unwrap();
        let ics = calendar(Method::Request, &[event()], now);
        assert!(ics.starts_with("BEGIN:VCALENDAR\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));
        for line in [
            "METHOD:REQUEST",
            "UID:abc@smart-learning-space",
            "SEQUENCE:2",
            "DTSTAMP:20261108T010203Z",
            "DTSTART:20261110T123000Z",
            "DTEND:20261110T140000Z",
            "SUMMARY:Toán 9\\; buổi 1\\, căn bậc hai",
            "STATUS:CONFIRMED",
            "TRIGGER:-PT15M",
        ] {
            assert!(ics.contains(&format!("{line}\r\n")), "thiếu {line}\n{ics}");
        }
        let cancel = calendar(Method::Cancel, &[event()], now);
        assert!(cancel.contains("METHOD:CANCEL\r\n") && cancel.contains("STATUS:CANCELLED\r\n"));
        assert!(!cancel.contains("VALARM"));
    }

    #[test]
    fn gap_dong_dai_khong_cat_giua_ky_tu() {
        let mut e = event();
        e.description = "Nội dung rất dài ".repeat(20);
        let ics = calendar(Method::Request, &[e], Utc::now());
        for line in ics.split("\r\n") {
            assert!(line.len() <= 75, "{line}");
        }
        // Ghép lại các dòng nối thì ra đúng nội dung.
        let unfolded = ics.replace("\r\n ", "");
        assert!(unfolded.contains(&"Nội dung rất dài ".repeat(20).replace(',', "\\,")));
    }
}

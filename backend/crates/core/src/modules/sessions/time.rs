//! Giờ Việt Nam (UTC+7, không đổi giờ mùa hè) và lịch lặp hằng tuần.

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};

/// Số buổi tối đa sinh ra từ một lịch lặp.
pub const MAX_OCCURRENCES: usize = 200;

pub fn vn() -> FixedOffset {
    FixedOffset::east_opt(7 * 3600).expect("UTC+7 hợp lệ")
}

pub fn to_local(t: DateTime<Utc>) -> NaiveDateTime {
    t.with_timezone(&vn()).naive_local()
}

pub fn from_local(t: NaiveDateTime) -> DateTime<Utc> {
    vn().from_local_datetime(&t).single().expect("UTC+7 không có giờ trùng / thiếu").with_timezone(&Utc)
}

const WEEKDAYS: [&str; 7] = ["thứ Hai", "thứ Ba", "thứ Tư", "thứ Năm", "thứ Sáu", "thứ Bảy", "Chủ nhật"];

/// "19:30 thứ Ba, 10/11/2026" (giờ Việt Nam).
pub fn format_vn(t: DateTime<Utc>) -> String {
    let l = to_local(t);
    format!("{} {}, {}", l.format("%H:%M"), WEEKDAYS[l.weekday().num_days_from_monday() as usize], l.format("%d/%m/%Y"))
}

/// Các giờ bắt đầu (UTC) của lịch lặp hằng tuần: mọi ngày trong [ngày của `first`, `until`] có thứ thuộc `weekdays`
/// (1 = thứ Hai … 7 = Chủ nhật), cùng giờ trong ngày với `first` (giờ Việt Nam).
/// `Err(n)` nếu vượt `MAX_OCCURRENCES`.
pub fn weekly(first: DateTime<Utc>, weekdays: &[u32], until: NaiveDate) -> Result<Vec<DateTime<Utc>>, usize> {
    let start = to_local(first);
    let mut out = Vec::new();
    let mut day = start.date();
    while day <= until {
        if weekdays.contains(&day.weekday().number_from_monday()) {
            out.push(from_local(day.and_time(start.time())));
            if out.len() > MAX_OCCURRENCES {
                return Err(out.len());
            }
        }
        day += Duration::days(1);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn lap_thu_ba_thu_nam_theo_gio_viet_nam() {
        // 19:30 thứ Ba 10/11/2026 giờ VN = 12:30 UTC.
        let first = utc("2026-11-10T12:30:00Z");
        let until = NaiveDate::from_ymd_opt(2026, 11, 24).unwrap();
        let got = weekly(first, &[2, 4], until).unwrap();
        let want: Vec<DateTime<Utc>> = [
            "2026-11-10T12:30:00Z",
            "2026-11-12T12:30:00Z",
            "2026-11-17T12:30:00Z",
            "2026-11-19T12:30:00Z",
            "2026-11-24T12:30:00Z",
        ]
        .iter()
        .map(|s| utc(s))
        .collect();
        assert_eq!(got, want);
    }

    #[test]
    fn ngay_bat_dau_khong_thuoc_thu_da_chon_va_gio_qua_nua_dem_utc() {
        // 06:00 thứ Hai 9/11 giờ VN = 23:00 Chủ nhật 8/11 UTC: thứ tính theo giờ VN.
        let first = utc("2026-11-08T23:00:00Z");
        let got = weekly(first, &[3], NaiveDate::from_ymd_opt(2026, 11, 18).unwrap()).unwrap();
        assert_eq!(got, vec![utc("2026-11-10T23:00:00Z"), utc("2026-11-17T23:00:00Z")]);
    }

    #[test]
    fn qua_nhieu_buoi_bi_tu_choi() {
        let first = utc("2026-01-05T12:00:00Z");
        assert!(weekly(first, &[1, 2, 3, 4, 5, 6, 7], NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()).is_err());
    }

    #[test]
    fn dinh_dang_gio_viet_nam() {
        assert_eq!(format_vn(utc("2026-11-10T12:30:00Z")), "19:30 thứ Ba, 10/11/2026");
    }
}

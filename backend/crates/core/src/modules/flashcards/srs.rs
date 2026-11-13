//! Lặp lại ngắt quãng (kiểu SM-2 / Anki, rút gọn). Hàm thuần, không đụng DB.
//!
//! - Quên: học lại sau 10 phút, giảm độ dễ, đếm số lần quên (nếu thẻ đã thuộc).
//! - Khó: khoảng cách × 1,2 (lần đầu 1 ngày), giảm độ dễ.
//! - Được: lần đầu 1 ngày, lần hai 3 ngày, sau đó × độ dễ.
//! - Dễ: lần đầu 4 ngày, sau đó × độ dễ × 1,3, tăng độ dễ.
//!
//! Khoảng cách luôn tăng ít nhất 1 ngày so với lần trước, tối đa 365 ngày; độ dễ trong [1,3; 3,0].

use chrono::{DateTime, TimeDelta, Utc};
use serde::{Deserialize, Serialize};

pub const START_EASE: f64 = 2.5;
pub const MIN_EASE: f64 = 1.3;
pub const MAX_EASE: f64 = 3.0;
pub const MAX_INTERVAL_DAYS: i32 = 365;
/// Quên → học lại sau bao nhiêu phút.
pub const RELEARN_MINUTES: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rating {
    Again,
    Hard,
    Good,
    Easy,
}

impl Rating {
    pub const ALL: [Rating; 4] = [Rating::Again, Rating::Hard, Rating::Good, Rating::Easy];

    pub fn as_str(self) -> &'static str {
        match self {
            Rating::Again => "again",
            Rating::Hard => "hard",
            Rating::Good => "good",
            Rating::Easy => "easy",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardState {
    /// Số lần nhớ liên tiếp (về 0 khi quên).
    pub repetitions: i32,
    pub interval_days: i32,
    pub ease: f64,
    pub lapses: i32,
}

impl Default for CardState {
    fn default() -> Self {
        Self { repetitions: 0, interval_days: 0, ease: START_EASE, lapses: 0 }
    }
}

/// Trạng thái mới + lúc cần ôn lại.
pub fn schedule(prev: Option<CardState>, rating: Rating, now: DateTime<Utc>) -> (CardState, DateTime<Utc>) {
    let p = prev.unwrap_or_default();
    let grow = |days: f64| ((days.round() as i32).max(p.interval_days + 1)).min(MAX_INTERVAL_DAYS);
    let next = match rating {
        Rating::Again => CardState {
            repetitions: 0,
            interval_days: 0,
            ease: (p.ease - 0.2).max(MIN_EASE),
            lapses: p.lapses + i32::from(p.repetitions > 0),
        },
        Rating::Hard => CardState {
            repetitions: p.repetitions + 1,
            interval_days: if p.repetitions == 0 { 1 } else { grow(f64::from(p.interval_days) * 1.2) },
            ease: (p.ease - 0.15).max(MIN_EASE),
            lapses: p.lapses,
        },
        Rating::Good => CardState {
            repetitions: p.repetitions + 1,
            interval_days: match p.repetitions {
                0 => 1,
                1 => 3.max(p.interval_days + 1),
                _ => grow(f64::from(p.interval_days) * p.ease),
            },
            ease: p.ease,
            lapses: p.lapses,
        },
        Rating::Easy => CardState {
            repetitions: p.repetitions + 1,
            interval_days: if p.repetitions == 0 { 4 } else { grow(f64::from(p.interval_days) * p.ease * 1.3) },
            ease: (p.ease + 0.15).min(MAX_EASE),
            lapses: p.lapses,
        },
    };
    let due = if next.interval_days == 0 {
        now + TimeDelta::minutes(RELEARN_MINUTES)
    } else {
        now + TimeDelta::days(i64::from(next.interval_days))
    };
    (next, due)
}

/// Thời gian tới lần ôn sau cho từng nút (phút), theo thứ tự Quên / Khó / Được / Dễ — để hiện trên nút.
pub fn preview_minutes(prev: Option<CardState>) -> [i64; 4] {
    let now = DateTime::<Utc>::UNIX_EPOCH;
    Rating::ALL.map(|r| (schedule(prev, r, now).1 - now).num_minutes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at() -> DateTime<Utc> {
        "2026-11-13T12:00:00Z".parse().unwrap()
    }

    #[test]
    fn the_moi_theo_tung_nut() {
        let now = at();
        let days = |r| {
            let (s, due) = schedule(None, r, now);
            (s.interval_days, (due - now).num_minutes())
        };
        assert_eq!(days(Rating::Again), (0, 10));
        assert_eq!(days(Rating::Hard), (1, 1440));
        assert_eq!(days(Rating::Good), (1, 1440));
        assert_eq!(days(Rating::Easy), (4, 4 * 1440));
        assert_eq!(preview_minutes(None), [10, 1440, 1440, 4 * 1440]);
    }

    #[test]
    fn nho_lien_tiep_khoang_cach_tang_dan() {
        let now = at();
        let (s1, _) = schedule(None, Rating::Good, now);
        let (s2, _) = schedule(Some(s1), Rating::Good, now);
        let (s3, _) = schedule(Some(s2), Rating::Good, now);
        let (s4, _) = schedule(Some(s3), Rating::Good, now);
        assert_eq!([s1.interval_days, s2.interval_days, s3.interval_days, s4.interval_days], [1, 3, 8, 20]);
        assert_eq!(s4.repetitions, 4);
    }

    #[test]
    fn quen_thi_hoc_lai_va_giam_do_de() {
        let now = at();
        let learned = CardState { repetitions: 3, interval_days: 8, ease: 2.5, lapses: 0 };
        let (s, due) = schedule(Some(learned), Rating::Again, now);
        assert_eq!((s.repetitions, s.interval_days, s.lapses), (0, 0, 1));
        assert!((s.ease - 2.3).abs() < 1e-9);
        assert_eq!((due - now).num_minutes(), 10);
        // Quên thẻ mới không tính là "quên lại".
        assert_eq!(schedule(None, Rating::Again, now).0.lapses, 0);
    }

    #[test]
    fn do_de_va_khoang_cach_co_gioi_han() {
        let now = at();
        let hard = CardState { repetitions: 5, interval_days: 10, ease: 1.35, lapses: 2 };
        let (s, _) = schedule(Some(hard), Rating::Hard, now);
        assert!((s.ease - MIN_EASE).abs() < 1e-9);
        assert_eq!(s.interval_days, 12);
        let long = CardState { repetitions: 9, interval_days: 300, ease: 3.0, lapses: 0 };
        let (s, _) = schedule(Some(long), Rating::Easy, now);
        assert_eq!(s.interval_days, MAX_INTERVAL_DAYS);
        assert!((s.ease - MAX_EASE).abs() < 1e-9);
    }
}

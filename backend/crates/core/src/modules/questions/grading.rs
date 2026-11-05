//! Chấm một câu (hàm thuần, không đụng DB). Chấm theo **ID phương án**, không theo vị trí A/B/C/D,
//! nên trộn thứ tự phương án không ảnh hưởng điểm.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::model::{QuestionBody, QuestionKind};

/// Cách chấm của một phần đề.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scoring {
    /// Đúng hoàn toàn mới được điểm câu.
    PerQuestion,
    /// Câu đúng/sai 4 ý theo đề THPT từ 2025: đúng 1 / 2 / 3 / 4 ý = 0,1 / 0,25 / 0,5 / 1 × điểm câu.
    ThptTrueFalse,
}

impl Scoring {
    pub fn as_str(self) -> &'static str {
        match self {
            Scoring::PerQuestion => "per_question",
            Scoring::ThptTrueFalse => "thpt_true_false",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "per_question" => Some(Self::PerQuestion),
            "thpt_true_false" => Some(Self::ThptTrueFalse),
            _ => None,
        }
    }
}

/// Câu trả lời của học sinh (lưu ở `attempt_items.answer`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    /// single / multiple: ID phương án đã chọn.
    #[serde(default)]
    pub selected: Vec<Uuid>,
    /// true_false: ID ý → học sinh chọn Đúng (`true`) hay Sai (`false`).
    #[serde(default)]
    pub truth: BTreeMap<Uuid, bool>,
    /// short: câu trả lời gõ vào.
    #[serde(default)]
    pub text: Option<String>,
}

impl Answer {
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty() && self.truth.is_empty() && self.text.as_deref().is_none_or(|t| t.trim().is_empty())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Graded {
    pub is_correct: bool,
    pub score: f64,
}

/// Làm tròn 2 chữ số thập phân (điểm hiển thị kiểu 7,25).
pub fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Chuẩn hóa câu trả lời ngắn: bỏ khoảng trắng thừa, chữ thường.
fn normalize_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Đọc số: `0,5` = `0.5`, `1/2`, `-3`, `2.50`. Không phải số → `None`.
fn parse_number(s: &str) -> Option<f64> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect::<String>().replace(',', ".");
    if s.is_empty() {
        return None;
    }
    if let Some((a, b)) = s.split_once('/') {
        let (a, b) = (a.parse::<f64>().ok()?, b.parse::<f64>().ok()?);
        return (b != 0.0).then_some(a / b);
    }
    s.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// Câu trả lời ngắn khớp một trong các đáp án chấp nhận (so số nếu cả hai là số, không thì so chữ).
pub fn short_matches(given: &str, accepted: &[String]) -> bool {
    let given_num = parse_number(given);
    let given_text = normalize_text(given);
    if given_text.is_empty() {
        return false;
    }
    accepted.iter().any(|a| match (given_num, parse_number(a)) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-9,
        _ => normalize_text(a) == given_text,
    })
}

/// Bảng điểm đúng/sai THPT theo số ý đúng (câu 4 ý).
const THPT_TF: [f64; 5] = [0.0, 0.1, 0.25, 0.5, 1.0];

pub fn grade(
    kind: QuestionKind,
    body: &QuestionBody,
    answer: Option<&Answer>,
    scoring: Scoring,
    points: f64,
) -> Graded {
    let empty = Answer::default();
    let answer = answer.unwrap_or(&empty);
    match kind {
        QuestionKind::Single | QuestionKind::Multiple => {
            let mut chosen = answer.selected.clone();
            chosen.sort();
            chosen.dedup();
            let mut correct = body.correct.clone();
            correct.sort();
            let ok = !correct.is_empty() && chosen == correct;
            Graded { is_correct: ok, score: if ok { points } else { 0.0 } }
        }
        QuestionKind::TrueFalse => {
            let total = body.options.len();
            let right =
                body.options.iter().filter(|o| answer.truth.get(&o.id) == Some(&body.correct.contains(&o.id))).count();
            let all = total > 0 && right == total;
            let score = match scoring {
                Scoring::PerQuestion => {
                    if all {
                        points
                    } else {
                        0.0
                    }
                }
                Scoring::ThptTrueFalse if total == 4 => THPT_TF[right] * points,
                // Câu không đủ 4 ý: chia theo tỉ lệ ý đúng.
                Scoring::ThptTrueFalse if total > 0 => right as f64 / total as f64 * points,
                Scoring::ThptTrueFalse => 0.0,
            };
            Graded { is_correct: all, score: round2(score) }
        }
        QuestionKind::Short => {
            let ok = answer.text.as_deref().is_some_and(|t| short_matches(t, &body.accepted));
            Graded { is_correct: ok, score: if ok { points } else { 0.0 } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::questions::model::QOption;

    fn body(n: usize, correct: &[usize]) -> (QuestionBody, Vec<Uuid>) {
        let ids: Vec<Uuid> = (0..n).map(|_| Uuid::now_v7()).collect();
        let body = QuestionBody {
            content: "Câu hỏi".into(),
            options: ids.iter().map(|id| QOption { id: *id, content: "x".into() }).collect(),
            correct: correct.iter().map(|i| ids[*i]).collect(),
            accepted: vec![],
            explanation: String::new(),
            shuffle_options: true,
        };
        (body, ids)
    }

    #[test]
    fn mot_dap_an_va_nhieu_dap_an_cham_theo_id() {
        let (b, ids) = body(4, &[2]);
        let pick = |sel: Vec<Uuid>| Answer { selected: sel, ..Default::default() };
        assert_eq!(grade(QuestionKind::Single, &b, Some(&pick(vec![ids[2]])), Scoring::PerQuestion, 0.25).score, 0.25);
        assert_eq!(grade(QuestionKind::Single, &b, Some(&pick(vec![ids[1]])), Scoring::PerQuestion, 0.25).score, 0.0);
        assert!(!grade(QuestionKind::Single, &b, None, Scoring::PerQuestion, 1.0).is_correct);

        let (b, ids) = body(4, &[0, 3]);
        let g = grade(QuestionKind::Multiple, &b, Some(&pick(vec![ids[3], ids[0]])), Scoring::PerQuestion, 1.0);
        assert!(g.is_correct && g.score == 1.0);
        // Thiếu hoặc thừa một phương án → 0 điểm.
        assert_eq!(grade(QuestionKind::Multiple, &b, Some(&pick(vec![ids[0]])), Scoring::PerQuestion, 1.0).score, 0.0);
        let extra = pick(vec![ids[0], ids[1], ids[3]]);
        assert_eq!(grade(QuestionKind::Multiple, &b, Some(&extra), Scoring::PerQuestion, 1.0).score, 0.0);
    }

    #[test]
    fn dung_sai_theo_thang_diem_thpt() {
        // Ý a, c đúng; b, d sai.
        let (b, ids) = body(4, &[0, 2]);
        let answer = |marks: [bool; 4]| Answer {
            truth: ids.iter().zip(marks).map(|(id, m)| (*id, m)).collect(),
            ..Default::default()
        };
        let score = |marks| grade(QuestionKind::TrueFalse, &b, Some(&answer(marks)), Scoring::ThptTrueFalse, 1.0).score;
        assert_eq!(score([true, false, true, false]), 1.0); // 4 ý
        assert_eq!(score([true, false, true, true]), 0.5); // 3 ý
        assert_eq!(score([true, true, true, true]), 0.25); // 2 ý
        assert_eq!(score([true, true, false, true]), 0.1); // 1 ý
        assert_eq!(score([false, true, false, true]), 0.0);
        // Bỏ trống một ý = ý đó sai.
        let partial = Answer { truth: [(ids[0], true), (ids[1], false), (ids[2], true)].into(), ..Default::default() };
        assert_eq!(grade(QuestionKind::TrueFalse, &b, Some(&partial), Scoring::ThptTrueFalse, 1.0).score, 0.5);
        // Chấm "đúng hết mới có điểm".
        assert_eq!(grade(QuestionKind::TrueFalse, &b, Some(&partial), Scoring::PerQuestion, 1.0).score, 0.0);
    }

    #[test]
    fn tra_loi_ngan_so_va_chu() {
        let accepted = vec!["0.5".to_string(), "Hà Nội".to_string()];
        for ok in ["0,5", " 0.50 ", "1/2", "hà   nội", "HÀ NỘI"] {
            assert!(short_matches(ok, &accepted), "{ok}");
        }
        for bad in ["0.51", "", "Ha Noi", "2/0"] {
            assert!(!short_matches(bad, &accepted), "{bad}");
        }
        let b = QuestionBody {
            content: "x".into(),
            options: vec![],
            correct: vec![],
            accepted: vec!["-3".into()],
            explanation: String::new(),
            shuffle_options: false,
        };
        let a = Answer { text: Some("-3,0".into()), ..Default::default() };
        assert_eq!(grade(QuestionKind::Short, &b, Some(&a), Scoring::PerQuestion, 0.5).score, 0.5);
    }

    #[test]
    fn lam_tron_hai_chu_so() {
        assert_eq!(round2(0.1 + 0.2), 0.3);
        assert_eq!(round2(7.256), 7.26);
    }
}

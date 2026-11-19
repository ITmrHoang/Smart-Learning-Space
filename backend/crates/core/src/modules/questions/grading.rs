//! Chấm một câu (hàm thuần, không đụng DB). Chấm theo **ID phương án**, không theo vị trí A/B/C/D,
//! nên trộn thứ tự phương án không ảnh hưởng điểm.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    expr,
    model::{MatchMode, QuestionBody, QuestionKind},
};

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
    /// short / essay: câu trả lời gõ vào.
    #[serde(default)]
    pub text: Option<String>,
    /// essay: ảnh / PDF bài làm (file mục đích `homework` của học sinh).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<Uuid>,
}

impl Answer {
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
            && self.truth.is_empty()
            && self.files.is_empty()
            && self.text.as_deref().is_none_or(|t| t.trim().is_empty())
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
    matches_with(given, accepted, None, MatchMode::Auto)
}

/// Như `short_matches`, thêm sai số cho đáp án là số và cách so biểu thức tương đương.
pub fn matches_with(given: &str, accepted: &[String], tolerance: Option<f64>, mode: MatchMode) -> bool {
    let given_num = parse_number(given);
    let given_text = normalize_text(given);
    if given_text.is_empty() {
        return false;
    }
    let tol = tolerance.unwrap_or(0.0).max(0.0) + 1e-9;
    let given_expr = (mode == MatchMode::Expression).then(|| expr::parse_answer(given).ok()).flatten();
    accepted.iter().any(|a| match (given_num, parse_number(a)) {
        (Some(x), Some(y)) => (x - y).abs() <= tol,
        _ if normalize_text(a) == given_text => true,
        _ => given_expr.as_ref().is_some_and(|g| expr::parse_answer(a).is_ok_and(|e| expr::equivalent(g, &e))),
    })
}

/// Điểm câu chấm tay: theo các bước của hướng dẫn chấm (quy về thang `points` của câu), hoặc điểm nhập thẳng khi câu
/// không có bước chấm. Kết quả nằm trong [0, points], làm tròn 2 chữ số.
pub fn manual_score(body: &QuestionBody, points: f64, marks: &BTreeMap<Uuid, f64>, direct: Option<f64>) -> f64 {
    let total = body.rubric_total();
    let raw = if total > 0.0 {
        let got: f64 = body.rubric.iter().map(|s| marks.get(&s.id).copied().unwrap_or(0.0).clamp(0.0, s.points)).sum();
        got / total * points
    } else {
        direct.unwrap_or(0.0)
    };
    round2(raw.clamp(0.0, points))
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
            let ok = answer
                .text
                .as_deref()
                .is_some_and(|t| matches_with(t, &body.accepted, body.tolerance, body.match_mode));
            Graded { is_correct: ok, score: if ok { points } else { 0.0 } }
        }
        // Giáo viên chấm tay (`manual_score`); chấm tự động để 0 điểm "chờ chấm".
        QuestionKind::Essay => Graded { is_correct: false, score: 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::questions::model::{QOption, RubricStep};

    fn body(n: usize, correct: &[usize]) -> (QuestionBody, Vec<Uuid>) {
        let ids: Vec<Uuid> = (0..n).map(|_| Uuid::now_v7()).collect();
        let body = QuestionBody {
            content: "Câu hỏi".into(),
            options: ids.iter().map(|id| QOption { id: *id, content: "x".into() }).collect(),
            correct: correct.iter().map(|i| ids[*i]).collect(),
            accepted: vec![],
            explanation: String::new(),
            shuffle_options: true,
            rubric: vec![],
            tolerance: None,
            match_mode: Default::default(),
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
            rubric: vec![],
            tolerance: None,
            match_mode: Default::default(),
        };
        let a = Answer { text: Some("-3,0".into()), ..Default::default() };
        assert_eq!(grade(QuestionKind::Short, &b, Some(&a), Scoring::PerQuestion, 0.5).score, 0.5);
    }

    #[test]
    fn tra_loi_ngan_sai_so_va_bieu_thuc() {
        let acc = vec!["3,14".to_string()];
        assert!(matches_with("3.15", &acc, Some(0.01), MatchMode::Auto));
        assert!(matches_with("3,13", &acc, Some(0.01), MatchMode::Auto));
        assert!(!matches_with("3.16", &acc, Some(0.01), MatchMode::Auto));
        assert!(!matches_with("3.15", &acc, None, MatchMode::Auto));

        let acc = vec!["2(x+1)".to_string()];
        assert!(matches_with("2x + 2", &acc, None, MatchMode::Expression));
        assert!(matches_with("2 + 2x", &acc, None, MatchMode::Expression));
        assert!(!matches_with("2x + 1", &acc, None, MatchMode::Expression));
        assert!(!matches_with("2x + 2", &acc, None, MatchMode::Auto), "so chữ thì phải viết giống");
        assert!(!matches_with("2x +", &acc, None, MatchMode::Expression), "không đọc được = sai");
        // Đáp án là số vẫn so số (có sai số) ở chế độ biểu thức
        assert!(matches_with("1/2", &["0,5".to_string()], None, MatchMode::Expression));
        assert!(matches_with("√2/2", &["1/√2".to_string()], None, MatchMode::Expression));
    }

    #[test]
    fn diem_tu_luan_theo_buoc() {
        let s = |p: f64| RubricStep { id: Uuid::now_v7(), content: "bước".into(), points: p };
        let mut b = QuestionBody {
            content: "Giải phương trình".into(),
            options: vec![],
            correct: vec![],
            accepted: vec![],
            explanation: String::new(),
            shuffle_options: false,
            rubric: vec![s(0.25), s(0.5), s(0.25)],
            tolerance: None,
            match_mode: Default::default(),
        };
        let ids: Vec<Uuid> = b.rubric.iter().map(|r| r.id).collect();
        let marks = |m: &[(usize, f64)]| m.iter().map(|(i, p)| (ids[*i], *p)).collect::<BTreeMap<_, _>>();
        // Câu 1 điểm, các bước tổng 1 điểm
        assert_eq!(manual_score(&b, 1.0, &marks(&[(0, 0.25), (1, 0.5)]), None), 0.75);
        // Đặt câu 2 điểm trong đề: quy theo tỉ lệ
        assert_eq!(manual_score(&b, 2.0, &marks(&[(0, 0.25), (1, 0.25)]), None), 1.0);
        // Cho quá điểm bước → kẹp lại; bước lạ bị bỏ qua
        let mut over = marks(&[(0, 5.0)]);
        over.insert(Uuid::now_v7(), 1.0);
        assert_eq!(manual_score(&b, 1.0, &over, None), 0.25);
        // Không có bước chấm: điểm nhập thẳng, kẹp trong [0, điểm câu]
        b.rubric.clear();
        assert_eq!(manual_score(&b, 1.5, &BTreeMap::new(), Some(1.25)), 1.25);
        assert_eq!(manual_score(&b, 1.5, &BTreeMap::new(), Some(9.0)), 1.5);
        assert_eq!(grade(QuestionKind::Essay, &b, None, Scoring::PerQuestion, 1.0).score, 0.0);
    }

    #[test]
    fn lam_tron_hai_chu_so() {
        assert_eq!(round2(0.1 + 0.2), 0.3);
        assert_eq!(round2(7.256), 7.26);
    }
}

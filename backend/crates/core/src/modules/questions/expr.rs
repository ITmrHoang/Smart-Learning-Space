//! Đọc biểu thức toán gõ tự nhiên và so hai biểu thức có **tương đương** không (câu trả lời ngắn, M38).
//!
//! Cách viết giống `hui-plot` ở giao diện: nhân ngầm (`2x`, `3(x+1)`, `xy`), dấu phẩy thập phân (`0,5`), `^`, `²` `³`,
//! `√x` / `sqrt(x)`, `|x|`, `sin cos tan tg cot cotg ln log exp abs`, `π` / `pi`, `×` `·` `÷` `:`. Không dùng `eval`.
//! So tương đương: thay các biến bằng nhiều bộ giá trị cố định rồi so kết quả (bỏ qua điểm biểu thức không xác định).

use std::collections::BTreeSet;

/// Biểu thức đã đọc.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Var(char),
    Neg(Box<Expr>),
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Call(Func, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Sqrt,
    Abs,
    Sin,
    Cos,
    Tan,
    Cot,
    Ln,
    Log,
    Exp,
}

const FUNCS: [(&str, Func); 11] = [
    ("sqrt", Func::Sqrt),
    ("abs", Func::Abs),
    ("sin", Func::Sin),
    ("cos", Func::Cos),
    ("tan", Func::Tan),
    ("tg", Func::Tan),
    ("cotg", Func::Cot),
    ("cot", Func::Cot),
    ("ln", Func::Ln),
    ("log", Func::Log),
    ("exp", Func::Exp),
];

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Var(char),
    Pi,
    Func(Func),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Bar,
    Sqrt,
    Sup(f64),
}

/// Độ sâu lồng ngoặc tối đa (chống chuỗi "((((((…" làm tràn ngăn xếp).
const MAX_DEPTH: usize = 64;

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '0'..='9' | '.' | ',' => {
                let start = i;
                let mut seen_sep = false;
                while i < chars.len() {
                    let d = chars[i];
                    if d.is_ascii_digit() {
                        i += 1;
                    } else if (d == '.' || d == ',')
                        && !seen_sep
                        && i + 1 < chars.len()
                        && chars[i + 1].is_ascii_digit()
                    {
                        seen_sep = true;
                        i += 1;
                    } else {
                        break;
                    }
                }
                if i == start {
                    return Err(format!("Ký tự \"{c}\" không đúng chỗ."));
                }
                let text: String = chars[start..i].iter().collect::<String>().replace(',', ".");
                out.push(Tok::Num(text.parse().map_err(|_| format!("Số \"{text}\" không hợp lệ."))?));
            }
            'π' => {
                out.push(Tok::Pi);
                i += 1;
            }
            c if c.is_alphabetic() => {
                let start = i;
                while i < chars.len() && chars[i].is_alphabetic() {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect::<String>().to_lowercase();
                read_word(&word, &mut out);
            }
            '+' => push(&mut out, &mut i, Tok::Plus),
            '-' | '−' | '–' => push(&mut out, &mut i, Tok::Minus),
            '*' | '×' | '·' | '⋅' => push(&mut out, &mut i, Tok::Star),
            '/' | '÷' | ':' => push(&mut out, &mut i, Tok::Slash),
            '^' => push(&mut out, &mut i, Tok::Caret),
            '(' | '[' | '{' => push(&mut out, &mut i, Tok::LParen),
            ')' | ']' | '}' => push(&mut out, &mut i, Tok::RParen),
            '|' => push(&mut out, &mut i, Tok::Bar),
            '√' => push(&mut out, &mut i, Tok::Sqrt),
            '²' => push(&mut out, &mut i, Tok::Sup(2.0)),
            '³' => push(&mut out, &mut i, Tok::Sup(3.0)),
            _ => return Err(format!("Ký tự \"{c}\" không dùng được trong biểu thức.")),
        }
    }
    Ok(out)
}

fn push(out: &mut Vec<Tok>, i: &mut usize, t: Tok) {
    out.push(t);
    *i += 1;
}

/// Một chữ: tên hàm / `pi`, hoặc các biến một chữ cái viết liền nhau (`xy` = `x·y`, `sinx` = `sin x`).
fn read_word(word: &str, out: &mut Vec<Tok>) {
    let mut rest = word;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("pi") {
            out.push(Tok::Pi);
            rest = r;
            continue;
        }
        if let Some((name, f)) = FUNCS.iter().find(|(name, _)| rest.starts_with(name)) {
            out.push(Tok::Func(*f));
            rest = &rest[name.len()..];
            continue;
        }
        let mut it = rest.chars();
        let ch = it.next().expect("rest không rỗng");
        out.push(Tok::Var(ch));
        rest = it.as_str();
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
    depth: usize,
    /// Đang đọc bên trong `|…|`: dấu `|` kế tiếp là dấu đóng, không phải nhân ngầm với `|…|` mới.
    in_abs: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    fn enter(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH { Err("Biểu thức lồng quá sâu.".into()) } else { Ok(()) }
    }

    fn expr(&mut self) -> Result<Expr, String> {
        self.enter()?;
        let mut left = self.term()?;
        while let Some(t) = self.peek() {
            left = match t {
                Tok::Plus => {
                    self.pos += 1;
                    Expr::Add(Box::new(left), Box::new(self.term()?))
                }
                Tok::Minus => {
                    self.pos += 1;
                    Expr::Sub(Box::new(left), Box::new(self.term()?))
                }
                _ => break,
            };
        }
        self.depth -= 1;
        Ok(left)
    }

    fn starts_operand(&self) -> bool {
        match self.peek() {
            Some(Tok::Num(_) | Tok::Var(_) | Tok::Pi | Tok::Func(_) | Tok::LParen | Tok::Sqrt) => true,
            Some(Tok::Bar) => self.in_abs == 0,
            _ => false,
        }
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            left = match self.peek() {
                Some(Tok::Star) => {
                    self.pos += 1;
                    Expr::Mul(Box::new(left), Box::new(self.unary()?))
                }
                Some(Tok::Slash) => {
                    self.pos += 1;
                    Expr::Div(Box::new(left), Box::new(self.unary()?))
                }
                // Nhân ngầm: 2x, 3(x + 1), x sin x
                _ if self.starts_operand() => Expr::Mul(Box::new(left), Box::new(self.power()?)),
                _ => break,
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Tok::Minus) => {
                self.pos += 1;
                self.enter()?;
                let e = Expr::Neg(Box::new(self.unary()?));
                self.depth -= 1;
                Ok(e)
            }
            Some(Tok::Plus) => {
                self.pos += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<Expr, String> {
        let mut base = self.primary()?;
        loop {
            match self.peek() {
                Some(Tok::Sup(n)) => {
                    let n = *n;
                    self.pos += 1;
                    base = Expr::Pow(Box::new(base), Box::new(Expr::Num(n)));
                }
                Some(Tok::Caret) => {
                    self.pos += 1;
                    self.enter()?;
                    let exp = self.unary()?; // 2^-1, x^(n+1); lũy thừa tính từ phải sang
                    self.depth -= 1;
                    return Ok(Expr::Pow(Box::new(base), Box::new(exp)));
                }
                _ => return Ok(base),
            }
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Var(c)) => Ok(Expr::Var(c)),
            Some(Tok::Pi) => Ok(Expr::Num(std::f64::consts::PI)),
            Some(Tok::LParen) => {
                let e = self.expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err("Thiếu dấu đóng ngoặc.".into()),
                }
            }
            Some(Tok::Bar) => {
                self.in_abs += 1;
                let e = self.expr()?;
                self.in_abs -= 1;
                match self.next() {
                    Some(Tok::Bar) => Ok(Expr::Call(Func::Abs, Box::new(e))),
                    _ => Err("Thiếu dấu | đóng giá trị tuyệt đối.".into()),
                }
            }
            Some(Tok::Sqrt) => Ok(Expr::Call(Func::Sqrt, Box::new(self.argument()?))),
            Some(Tok::Func(f)) => Ok(Expr::Call(f, Box::new(self.argument()?))),
            Some(_) => Err("Biểu thức viết chưa đúng.".into()),
            None => Err("Biểu thức chưa viết xong.".into()),
        }
    }

    /// Đối số của hàm / căn: `sin(x)^2` = (sin x)², còn không có ngoặc thì lấy một lũy thừa: `sin x^2` = sin(x²).
    fn argument(&mut self) -> Result<Expr, String> {
        self.enter()?;
        let e = if self.peek() == Some(&Tok::LParen) { self.primary()? } else { self.power()? };
        self.depth -= 1;
        Ok(e)
    }
}

/// Đọc một biểu thức (không có dấu `=`).
pub fn parse(s: &str) -> Result<Expr, String> {
    let toks = tokenize(s)?;
    if toks.is_empty() {
        return Err("Biểu thức trống.".into());
    }
    let mut p = Parser { toks, pos: 0, depth: 0, in_abs: 0 };
    let e = p.expr()?;
    if p.pos < p.toks.len() {
        return Err("Biểu thức viết chưa đúng.".into());
    }
    Ok(e)
}

/// Biểu thức hoặc phương trình `trái = phải` (đổi thành `trái − phải`).
#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Expr(Expr),
    Equation(Expr),
}

pub fn parse_answer(s: &str) -> Result<Parsed, String> {
    let parts: Vec<&str> = s.split('=').collect();
    match parts.as_slice() {
        [one] => Ok(Parsed::Expr(parse(one)?)),
        [l, r] => Ok(Parsed::Equation(Expr::Sub(Box::new(parse(l)?), Box::new(parse(r)?)))),
        _ => Err("Chỉ được có một dấu =.".into()),
    }
}

impl Expr {
    fn vars(&self, out: &mut BTreeSet<char>) {
        match self {
            Expr::Num(_) => {}
            Expr::Var(c) => {
                out.insert(*c);
            }
            Expr::Neg(a) | Expr::Call(_, a) => a.vars(out),
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b) | Expr::Div(a, b) | Expr::Pow(a, b) => {
                a.vars(out);
                b.vars(out);
            }
        }
    }

    /// Giá trị với `vals[i]` là giá trị của biến `names[i]`.
    pub fn eval(&self, names: &[char], vals: &[f64]) -> f64 {
        let ev = |e: &Expr| e.eval(names, vals);
        match self {
            Expr::Num(n) => *n,
            Expr::Var(c) => names.iter().position(|n| n == c).map_or(f64::NAN, |i| vals[i]),
            Expr::Neg(a) => -ev(a),
            Expr::Add(a, b) => ev(a) + ev(b),
            Expr::Sub(a, b) => ev(a) - ev(b),
            Expr::Mul(a, b) => ev(a) * ev(b),
            Expr::Div(a, b) => ev(a) / ev(b),
            Expr::Pow(a, b) => ev(a).powf(ev(b)),
            Expr::Call(f, a) => {
                let x = ev(a);
                match f {
                    Func::Sqrt => x.sqrt(),
                    Func::Abs => x.abs(),
                    Func::Sin => x.sin(),
                    Func::Cos => x.cos(),
                    Func::Tan => x.tan(),
                    Func::Cot => 1.0 / x.tan(),
                    Func::Ln => x.ln(),
                    Func::Log => x.log10(),
                    Func::Exp => x.exp(),
                }
            }
        }
    }
}

/// Giá trị thử cho biến thứ `k` ở lần thử thứ `i` (cố định để chấm lại vẫn ra cùng kết quả).
fn sample(i: usize, k: usize) -> f64 {
    const BASE: [f64; 10] = [0.731, 1.917, 2.383, -0.613, 3.271, 0.419, -1.847, 1.237, 2.903, -2.311];
    BASE[(i * 3 + k * 7) % BASE.len()] + k as f64 * 0.173
}

const SAMPLES: usize = 10;
/// Cần ít nhất chừng này lần thử mà cả hai biểu thức đều xác định.
const MIN_VALID: usize = 3;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-7 * a.abs().max(b.abs()).max(1.0)
}

/// Hai câu trả lời tương đương: cùng là biểu thức và bằng nhau, hoặc cùng là phương trình và tỉ lệ với nhau
/// (`x + 1 = 0` ~ `2x + 2 = 0` ~ `-x = 1`).
pub fn equivalent(a: &Parsed, b: &Parsed) -> bool {
    let (ea, eb, equation) = match (a, b) {
        (Parsed::Expr(x), Parsed::Expr(y)) => (x, y, false),
        (Parsed::Equation(x), Parsed::Equation(y)) => (x, y, true),
        _ => return false,
    };
    let mut names = BTreeSet::new();
    ea.vars(&mut names);
    eb.vars(&mut names);
    let names: Vec<char> = names.into_iter().collect();
    let mut valid = 0;
    let mut ratio: Option<f64> = None;
    for i in 0..SAMPLES {
        let vals: Vec<f64> = (0..names.len()).map(|k| sample(i, k)).collect();
        let (x, y) = (ea.eval(&names, &vals), eb.eval(&names, &vals));
        match (x.is_finite(), y.is_finite()) {
            (false, false) => continue,
            (true, true) => {}
            _ => return false,
        }
        if equation {
            if x.abs() < 1e-9 && y.abs() < 1e-9 {
                valid += 1;
                continue;
            }
            if x.abs() < 1e-9 || y.abs() < 1e-9 {
                return false;
            }
            let r = x / y;
            match ratio {
                Some(r0) if !close(r0, r) => return false,
                None => ratio = Some(r),
                _ => {}
            }
        } else if !close(x, y) {
            return false;
        }
        valid += 1;
        if names.is_empty() {
            return true;
        }
    }
    valid >= MIN_VALID
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eq(a: &str, b: &str) -> bool {
        equivalent(&parse_answer(a).unwrap(), &parse_answer(b).unwrap())
    }

    #[test]
    fn bieu_thuc_tuong_duong() {
        for (a, b) in [
            ("2(x+1)", "2x + 2"),
            ("x^2 - 1", "(x-1)(x+1)"),
            ("x²-2x+1", "(x − 1)^2"),
            ("1/2", "0,5"),
            ("√2/2", "1/√2"),
            ("sqrt(x^2)", "|x|"),
            ("2πr", "2·r·pi"),
            ("xy + x", "x(y+1)"),
            ("sin(x)^2", "1 - cos(x)^2"),
            ("sin x^2", "sin(x²)"),
            ("sin x / cos x", "tg x"),
            ("a/b + c/b", "(a + c) : b"),
            ("2^-1", "1/2"),
            ("ln(x²)", "2ln|x|"),
            ("[x+1]·{x-1}", "x^2 - 1"),
        ] {
            assert!(eq(a, b), "{a} ~ {b}");
        }
    }

    #[test]
    fn bieu_thuc_khac_nhau() {
        for (a, b) in [
            ("2x + 2", "2x + 1"),
            ("x^2", "2x"),
            ("(x-1)^2", "x^2 - 1"),
            ("sqrt(x)", "x"),
            // Khác tập xác định: x < 0 thì vế trái không xác định
            ("sqrt(x)^2", "x"),
            ("sin(x)^2", "sin(x^2)"),
            ("x + y", "x + 2y"),
            ("1/3", "0,33"),
            ("x", "x = 0"),
        ] {
            assert!(!eq(a, b), "{a} !~ {b}");
        }
    }

    #[test]
    fn phuong_trinh_ti_le() {
        assert!(eq("x + 1 = 0", "2x + 2 = 0"));
        assert!(eq("y = 2x + 1", "2x - y + 1 = 0"));
        assert!(eq("x = 2", "2 = x"));
        assert!(!eq("x = 2", "x = 3"));
        assert!(!eq("y = 2x + 1", "y = x + 1"));
    }

    #[test]
    fn loi_doc_bieu_thuc() {
        for bad in ["", "2 +", "(x + 1", "x = 1 = 2", "2 # 3", "|x", ")(", "sin"] {
            assert!(parse_answer(bad).is_err(), "{bad}");
        }
        let deep = format!("{}x{}", "(".repeat(200), ")".repeat(200));
        assert!(parse(&deep).is_err());
    }

    #[test]
    fn doc_dung_thu_tu_phep_tinh() {
        let v = |s: &str| parse(s).unwrap().eval(&[], &[]);
        assert_eq!(v("2 + 3 · 4"), 14.0);
        assert_eq!(v("-2^2"), -4.0);
        assert_eq!(v("2^3^2"), 512.0);
        assert_eq!(v("6 : 2(1 + 2)"), 9.0);
        assert_eq!(v("3²"), 9.0);
        assert_eq!(v("|−5| + √9"), 8.0);
        assert_eq!(v("1,5 + 0.5"), 2.0);
    }
}

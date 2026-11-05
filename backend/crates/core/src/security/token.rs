//! Token ngẫu nhiên (refresh token, link đặt lại mật khẩu) và mã OTP.
//! DB chỉ lưu **hash SHA-256**, không lưu token gốc.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

/// 32 byte ngẫu nhiên từ hệ điều hành, mã hóa base64 an toàn cho URL (43 ký tự).
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("hệ điều hành cung cấp số ngẫu nhiên");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Mã OTP 6 chữ số, phân bố đều (loại bỏ phần dư để không bị lệch).
pub fn random_otp() -> String {
    const RANGE: u32 = 1_000_000;
    let limit = u32::MAX - (u32::MAX % RANGE);
    loop {
        let n = getrandom::u32().expect("hệ điều hành cung cấp số ngẫu nhiên");
        if n < limit {
            return format!("{:06}", n % RANGE);
        }
    }
}

/// Số ngẫu nhiên đều trong `0..n` (n > 0).
fn random_below(n: u32) -> u32 {
    let limit = u32::MAX - (u32::MAX % n);
    loop {
        let x = getrandom::u32().expect("hệ điều hành cung cấp số ngẫu nhiên");
        if x < limit {
            return x % n;
        }
    }
}

/// Trộn ngẫu nhiên tại chỗ (Fisher–Yates) — trộn câu hỏi / phương án cho từng lượt làm bài.
pub fn shuffle<T>(items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let j = random_below(i as u32 + 1) as usize;
        items.swap(i, j);
    }
}

/// Mã mời lớp học: 8 ký tự dễ đọc (bỏ 0, O, 1, I, L).
pub fn random_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
    (0..8).map(|_| ALPHABET[random_below(ALPHABET.len() as u32) as usize] as char).collect()
}

pub fn sha256_hex(input: &str) -> String {
    hex::encode(Sha256::digest(input.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_luon_du_6_chu_so() {
        for _ in 0..1000 {
            let otp = random_otp();
            assert_eq!(otp.len(), 6);
            assert!(otp.chars().all(|c| c.is_ascii_digit()));
        }
    }

    #[test]
    fn token_ngau_nhien_khac_nhau() {
        let (a, b) = (random_token(), random_token());
        assert_eq!(a.len(), 43);
        assert_ne!(a, b);
    }

    #[test]
    fn sha256_on_dinh() {
        assert_eq!(sha256_hex("abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }
}

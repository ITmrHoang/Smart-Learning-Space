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

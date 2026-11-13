//! Gửi email qua SMTP. Cấu hình đọc từ biến môi trường:
//!
//! | Biến | Mặc định | Ghi chú |
//! |---|---|---|
//! | `SMTP_TLS` | `starttls` | `starttls` (cổng 587) · `tls` (TLS ngay từ đầu, cổng 465) · `none` (không mã hóa — chỉ dùng cho Mailpit khi dev) |
//! | `SMTP_HOST` | `127.0.0.1` | Cũng là tên dùng để kiểm tra chứng chỉ TLS |
//! | `SMTP_PORT` | theo `SMTP_TLS`: 587 · 465 · 1025 | |
//! | `SMTP_USERNAME`, `SMTP_PASSWORD` | không có | Có cả hai thì mới đăng nhập |
//! | `SMTP_FROM` | `Smart Learning Space <no-reply@sls.local>` | |
//! | `SMTP_CA_FILE` | không có | File PEM chứng chỉ CA thêm vào danh sách tin cậy (máy chủ mail nội bộ dùng CA riêng) |
//!
//! Chứng chỉ được kiểm tra bằng bộ CA công khai gói sẵn trong binary (`webpki-roots`), nên image distroless
//! không cần cài thêm chứng chỉ của hệ điều hành.

use std::{fmt, time::Duration};

use anyhow::{Context, bail};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Attachment as MailAttachment, Mailbox, MultiPart, SinglePart, header::ContentType},
    transport::smtp::{
        authentication::Credentials,
        client::{Certificate, Tls, TlsParameters},
    },
};
use sls_core::jobs::Email;

/// Cách mã hóa kết nối tới máy chủ SMTP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpSecurity {
    /// Kết nối thường rồi bắt buộc nâng lên TLS bằng lệnh STARTTLS (cổng 587). Máy chủ không hỗ trợ thì báo lỗi.
    StartTls,
    /// TLS ngay từ đầu (SMTPS, cổng 465).
    Tls,
    /// Không mã hóa. Chỉ dùng với Mailpit / máy chủ trên chính máy này.
    None,
}

impl SmtpSecurity {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "starttls" => Some(Self::StartTls),
            "tls" | "ssl" | "smtps" => Some(Self::Tls),
            "none" | "off" | "plain" => Some(Self::None),
            _ => None,
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::StartTls => 587,
            Self::Tls => 465,
            Self::None => 1025,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub security: SmtpSecurity,
    pub credentials: Option<(String, String)>,
    pub from: String,
    pub ca_file: Option<String>,
}

impl fmt::Debug for SmtpConfig {
    // Viết tay để không in mật khẩu ra log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmtpConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("security", &self.security)
            .field("username", &self.credentials.as_ref().map(|(u, _)| u))
            .field("from", &self.from)
            .field("ca_file", &self.ca_file)
            .finish()
    }
}

impl SmtpConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Self::from_lookup(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()))
    }

    /// Tách riêng hàm đọc biến để test được mà không phải sửa biến môi trường thật.
    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> anyhow::Result<Self> {
        let security = match get("SMTP_TLS") {
            None => SmtpSecurity::StartTls,
            Some(v) => SmtpSecurity::parse(&v)
                .with_context(|| format!("SMTP_TLS không hợp lệ: `{v}` (dùng starttls, tls hoặc none)"))?,
        };
        let port = match get("SMTP_PORT") {
            None => security.default_port(),
            Some(v) => v.trim().parse().with_context(|| format!("SMTP_PORT không hợp lệ: `{v}`"))?,
        };
        let credentials = match (get("SMTP_USERNAME"), get("SMTP_PASSWORD")) {
            (Some(user), Some(pass)) => Some((user, pass)),
            (None, None) => None,
            _ => bail!("cần đặt cả SMTP_USERNAME và SMTP_PASSWORD (hoặc bỏ trống cả hai)"),
        };
        Ok(Self {
            host: get("SMTP_HOST").unwrap_or_else(|| "127.0.0.1".into()),
            port,
            security,
            credentials,
            from: get("SMTP_FROM").unwrap_or_else(|| "Smart Learning Space <no-reply@sls.local>".into()),
            ca_file: get("SMTP_CA_FILE"),
        })
    }
}

pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl Mailer {
    pub fn new(config: &SmtpConfig) -> anyhow::Result<Self> {
        let from = config.from.parse().context("SMTP_FROM không hợp lệ")?;
        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
            .port(config.port)
            .timeout(Some(Duration::from_secs(30)));

        if config.security != SmtpSecurity::None {
            let mut params = TlsParameters::builder(config.host.clone());
            if let Some(path) = &config.ca_file {
                let pem = std::fs::read(path).with_context(|| format!("không đọc được SMTP_CA_FILE `{path}`"))?;
                params = params.add_root_certificate(
                    Certificate::from_pem(&pem)
                        .with_context(|| format!("SMTP_CA_FILE `{path}` không phải PEM hợp lệ"))?,
                );
            }
            let params = params.build().context("không tạo được cấu hình TLS cho SMTP")?;
            builder = builder.tls(match config.security {
                SmtpSecurity::Tls => Tls::Wrapper(params),
                _ => Tls::Required(params),
            });
        } else if config.credentials.is_some() {
            tracing::warn!(
                host = %config.host,
                "SMTP_TLS=none nhưng có mật khẩu SMTP: mật khẩu sẽ đi qua mạng không mã hóa"
            );
        }

        if let Some((user, pass)) = &config.credentials {
            builder = builder.credentials(Credentials::new(user.clone(), pass.clone()));
        }
        Ok(Self { transport: builder.build(), from })
    }

    pub async fn send(&self, email: &Email) -> anyhow::Result<()> {
        let builder = Message::builder()
            .from(self.from.clone())
            .to(email.to.parse().context("địa chỉ nhận không hợp lệ")?)
            .subject(&email.subject);
        let message = if email.attachments.is_empty() {
            builder.header(ContentType::TEXT_PLAIN).body(email.text.clone())?
        } else {
            // Nội dung chữ + file đính kèm (lịch .ics…)
            let mut parts = MultiPart::mixed().singlepart(SinglePart::plain(email.text.clone()));
            for a in &email.attachments {
                let content_type = ContentType::parse(&a.content_type).context("content type đính kèm không hợp lệ")?;
                parts = parts.singlepart(MailAttachment::new(a.filename.clone()).body(a.content.clone(), content_type));
            }
            builder.multipart(parts)?
        };
        self.transport.send(message).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn config(vars: &[(&str, &str)]) -> anyhow::Result<SmtpConfig> {
        let map: HashMap<String, String> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        SmtpConfig::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn mac_dinh_la_starttls_cong_587() {
        let c = config(&[("SMTP_HOST", "smtp.gmail.com")]).unwrap();
        assert_eq!(c.security, SmtpSecurity::StartTls);
        assert_eq!(c.port, 587);
        assert_eq!(c.credentials, None);
    }

    #[test]
    fn cong_mac_dinh_theo_kieu_tls() {
        assert_eq!(config(&[("SMTP_TLS", "tls")]).unwrap().port, 465);
        assert_eq!(config(&[("SMTP_TLS", "none")]).unwrap().port, 1025);
        assert_eq!(config(&[("SMTP_TLS", "STARTTLS"), ("SMTP_PORT", "2525")]).unwrap().port, 2525);
    }

    #[test]
    fn doc_ten_khac_cua_kieu_tls() {
        assert_eq!(SmtpSecurity::parse("SSL"), Some(SmtpSecurity::Tls));
        assert_eq!(SmtpSecurity::parse(" smtps "), Some(SmtpSecurity::Tls));
        assert_eq!(SmtpSecurity::parse("off"), Some(SmtpSecurity::None));
    }

    #[test]
    fn bao_loi_gia_tri_sai() {
        assert!(config(&[("SMTP_TLS", "yes")]).is_err());
        assert!(config(&[("SMTP_PORT", "abc")]).is_err());
    }

    #[test]
    fn can_ca_ten_dang_nhap_va_mat_khau() {
        assert!(config(&[("SMTP_USERNAME", "a")]).is_err());
        assert!(config(&[("SMTP_PASSWORD", "b")]).is_err());
        let c = config(&[("SMTP_USERNAME", "a"), ("SMTP_PASSWORD", "b")]).unwrap();
        assert_eq!(c.credentials, Some(("a".into(), "b".into())));
    }

    #[test]
    fn debug_khong_in_mat_khau() {
        let c = config(&[("SMTP_USERNAME", "user"), ("SMTP_PASSWORD", "bi-mat-123")]).unwrap();
        let s = format!("{c:?}");
        assert!(s.contains("user"));
        assert!(!s.contains("bi-mat-123"));
    }

    #[test]
    fn ca_file_khong_doc_duoc_thi_bao_loi() {
        let c = config(&[("SMTP_HOST", "mail.local"), ("SMTP_CA_FILE", "/khong/co/file.pem")]).unwrap();
        assert!(Mailer::new(&c).is_err());
    }

    #[test]
    fn tao_duoc_mailer_cho_moi_kieu() {
        for tls in ["starttls", "tls", "none"] {
            let c = config(&[("SMTP_HOST", "smtp.example.com"), ("SMTP_TLS", tls)]).unwrap();
            assert!(Mailer::new(&c).is_ok(), "SMTP_TLS={tls}");
        }
    }

    /// Gửi thật một email theo cấu hình `SMTP_*` đang đặt trong biến môi trường. Không chạy mặc định:
    ///   SMTP_TLS=starttls SMTP_HOST=… SMTP_TEST_TO=ban@example.com cargo test -p sls-worker -- --ignored
    #[tokio::test]
    #[ignore = "cần máy chủ SMTP thật; đặt SMTP_* và SMTP_TEST_TO"]
    async fn gui_that_theo_bien_moi_truong() {
        let to = std::env::var("SMTP_TEST_TO").expect("đặt SMTP_TEST_TO = địa chỉ nhận thử");
        let config = SmtpConfig::from_env().unwrap();
        let mailer = Mailer::new(&config).unwrap();
        let email = Email {
            to,
            subject: format!("Thử SMTP ({:?})", config.security),
            text: "Email thử từ sls-worker.".into(),
            ..Default::default()
        };
        mailer.send(&email).await.unwrap();
    }
}

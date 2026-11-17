//! Lưu file trên Garage (tương thích S3) — `analyst.md` mục "Garage chạy ở đâu".
//!
//! - Trình duyệt upload / tải **thẳng** với Garage bằng presigned URL (API không chuyển dữ liệu file).
//!   Caddy (production) và Vite (dev) chuyển `/files/*` vào Garage; bucket tên `files` nên URL
//!   path-style `{S3_PUBLIC_URL}/files/{key}` dùng chung domain + HTTPS với ứng dụng.
//! - API chỉ gọi Garage để kiểm tra file sau khi upload (HEAD + đọc vài byte đầu) và xóa file.
//! - Ký request theo AWS Signature V4, viết tay (HMAC-SHA256 trên `sha2`) để không thêm crate.
//! - Test dùng kho trong bộ nhớ (`Storage::memory`).

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Utc};
use reqwest::{Method, Url};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};

/// Hash SHA-256 của body rỗng (request HEAD / GET / DELETE do API gửi).
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[derive(Clone)]
pub struct StorageConfig {
    /// Địa chỉ API gọi Garage (dev `http://127.0.0.1:3900`, production `http://garage:3900`).
    pub endpoint: String,
    /// Gốc URL trình duyệt dùng (mặc định = `APP_BASE_URL`; `/files/*` được chuyển vào Garage).
    pub public_url: String,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
}

/// Thông tin file trên kho (HEAD).
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub size: i64,
    pub content_type: Option<String>,
}

/// Kho trong bộ nhớ: khóa → (nội dung, Content-Type).
type MemoryObjects = Arc<Mutex<HashMap<String, (Vec<u8>, String)>>>;

#[derive(Clone)]
pub enum Storage {
    /// Chưa cấu hình `S3_*` → upload báo lỗi `STORAGE_DISABLED`.
    Disabled,
    S3(Arc<S3Store>),
    /// Kho trong bộ nhớ cho test: URL ký giả, đọc / ghi qua `memory_put`.
    Memory(MemoryObjects),
}

pub struct S3Store {
    cfg: StorageConfig,
    http: reqwest::Client,
}

fn disabled() -> AppError {
    AppError::new(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "STORAGE_DISABLED",
        "Chưa cấu hình nơi lưu file (Garage). Hãy báo quản trị viên.",
    )
}

impl Storage {
    pub fn from_config(cfg: Option<&StorageConfig>) -> Self {
        match cfg {
            Some(c) => Self::S3(Arc::new(S3Store {
                cfg: c.clone(),
                http: reqwest::Client::builder()
                    .timeout(Duration::from_secs(20))
                    .build()
                    .expect("tạo HTTP client cho Garage"),
            })),
            None => Self::Disabled,
        }
    }

    pub fn memory() -> Self {
        Self::Memory(Arc::new(Mutex::new(HashMap::new())))
    }

    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::Disabled)
    }

    /// Test: giả lập trình duyệt đã upload file lên kho.
    pub fn memory_put(&self, key: &str, bytes: &[u8], content_type: &str) {
        if let Self::Memory(m) = self {
            m.lock().unwrap().insert(key.to_string(), (bytes.to_vec(), content_type.to_string()));
        }
    }

    /// URL để trình duyệt `PUT` file (phải gửi đúng header `Content-Type` đã ký).
    pub fn presign_put(&self, key: &str, content_type: &str, expires: Duration) -> AppResult<String> {
        match self {
            Self::Disabled => Err(disabled()),
            Self::Memory(_) => Ok(format!("memory://files/{key}?put")),
            Self::S3(s) => s.presign(Method::PUT, key, Some(content_type), &[], expires, Utc::now()),
        }
    }

    /// URL để trình duyệt tải / hiện file. `Content-Type` và tên file khi tải do API ép (không tin
    /// giá trị lúc upload) để file không bị trình duyệt hiểu thành trang HTML.
    pub fn presign_get(
        &self,
        key: &str,
        content_type: &str,
        file_name: &str,
        inline: bool,
        expires: Duration,
    ) -> AppResult<String> {
        let disposition = format!(
            "{}; filename*=UTF-8''{}",
            if inline { "inline" } else { "attachment" },
            uri_encode(file_name, true)
        );
        match self {
            Self::Disabled => Err(disabled()),
            Self::Memory(_) => Ok(format!("memory://files/{key}?get")),
            Self::S3(s) => s.presign(
                Method::GET,
                key,
                None,
                &[("response-content-disposition", &disposition), ("response-content-type", content_type)],
                expires,
                Utc::now(),
            ),
        }
    }

    pub async fn head(&self, key: &str) -> AppResult<Option<ObjectInfo>> {
        match self {
            Self::Disabled => Err(disabled()),
            Self::Memory(m) => Ok(m
                .lock()
                .unwrap()
                .get(key)
                .map(|(b, ct)| ObjectInfo { size: b.len() as i64, content_type: Some(ct.clone()) })),
            Self::S3(s) => {
                let res = s.send(Method::HEAD, key, None).await?;
                if res.status() == reqwest::StatusCode::NOT_FOUND {
                    return Ok(None);
                }
                if !res.status().is_success() {
                    return Err(AppError::internal(format!("Garage HEAD {key}: {}", res.status())));
                }
                let header = |name: &str| res.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
                Ok(Some(ObjectInfo {
                    size: header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0),
                    content_type: header("content-type"),
                }))
            }
        }
    }

    /// Đọc `n` byte đầu của file (kiểm tra chữ ký định dạng ảnh / PDF).
    pub async fn read_prefix(&self, key: &str, n: usize) -> AppResult<Vec<u8>> {
        match self {
            Self::Disabled => Err(disabled()),
            Self::Memory(m) => {
                Ok(m.lock().unwrap().get(key).map(|(b, _)| b[..b.len().min(n)].to_vec()).unwrap_or_default())
            }
            Self::S3(s) => {
                let range = format!("bytes=0-{}", n.saturating_sub(1));
                let res = s.send(Method::GET, key, Some(&range)).await?;
                if !res.status().is_success() {
                    return Err(AppError::internal(format!("Garage GET {key}: {}", res.status())));
                }
                let bytes = res.bytes().await.map_err(AppError::internal)?;
                Ok(bytes[..bytes.len().min(n)].to_vec())
            }
        }
    }

    /// Đọc cả file (tối đa `max` byte — lớn hơn thì báo lỗi, không đọc tiếp).
    pub async fn get(&self, key: &str, max: usize) -> AppResult<Vec<u8>> {
        let too_big = || AppError::field("file", format!("File tối đa {} MB.", max / 1024 / 1024));
        match self {
            Self::Disabled => Err(disabled()),
            Self::Memory(m) => {
                let bytes = m.lock().unwrap().get(key).map(|(b, _)| b.clone()).unwrap_or_default();
                if bytes.len() > max { Err(too_big()) } else { Ok(bytes) }
            }
            Self::S3(s) => {
                let mut res = s.send(Method::GET, key, None).await?;
                if !res.status().is_success() {
                    return Err(AppError::internal(format!("Garage GET {key}: {}", res.status())));
                }
                let mut out = Vec::new();
                while let Some(chunk) = res.chunk().await.map_err(AppError::internal)? {
                    if out.len() + chunk.len() > max {
                        return Err(too_big());
                    }
                    out.extend_from_slice(&chunk);
                }
                Ok(out)
            }
        }
    }

    /// API tự ghi file lên kho (ảnh lấy ra từ file Word…).
    pub async fn put(&self, key: &str, bytes: &[u8], content_type: &str) -> AppResult<()> {
        match self {
            Self::Disabled => Err(disabled()),
            Self::Memory(_) => {
                self.memory_put(key, bytes, content_type);
                Ok(())
            }
            Self::S3(s) => {
                let res = s.put(key, bytes, content_type).await?;
                if res.status().is_success() {
                    Ok(())
                } else {
                    Err(AppError::internal(format!("Garage PUT {key}: {}", res.status())))
                }
            }
        }
    }

    pub async fn delete(&self, key: &str) -> AppResult<()> {
        match self {
            Self::Disabled => Err(disabled()),
            Self::Memory(m) => {
                m.lock().unwrap().remove(key);
                Ok(())
            }
            Self::S3(s) => {
                let res = s.send(Method::DELETE, key, None).await?;
                if res.status().is_success() || res.status() == reqwest::StatusCode::NOT_FOUND {
                    Ok(())
                } else {
                    Err(AppError::internal(format!("Garage DELETE {key}: {}", res.status())))
                }
            }
        }
    }
}

impl S3Store {
    fn object_url(&self, base: &str, key: &str) -> AppResult<Url> {
        let path = format!("{}/{}", uri_encode(&self.cfg.bucket, true), uri_encode(key, false));
        Url::parse(&format!("{}/{path}", base.trim_end_matches('/'))).map_err(AppError::internal)
    }

    fn presign(
        &self,
        method: Method,
        key: &str,
        content_type: Option<&str>,
        extra: &[(&str, &str)],
        expires: Duration,
        now: DateTime<Utc>,
    ) -> AppResult<String> {
        let url = self.object_url(&self.cfg.public_url, key)?;
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let scope = format!("{}/{}/s3/aws4_request", &amz_date[..8], self.cfg.region);
        let mut headers = vec![("host".to_string(), host_header(&url))];
        if let Some(ct) = content_type {
            headers.insert(0, ("content-type".to_string(), ct.to_string()));
        }
        let signed_headers = headers.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>().join(";");
        let mut query: Vec<(String, String)> = vec![
            ("X-Amz-Algorithm".into(), "AWS4-HMAC-SHA256".into()),
            ("X-Amz-Credential".into(), format!("{}/{scope}", self.cfg.access_key)),
            ("X-Amz-Date".into(), amz_date.clone()),
            ("X-Amz-Expires".into(), expires.as_secs().to_string()),
            ("X-Amz-SignedHeaders".into(), signed_headers.clone()),
        ];
        query.extend(extra.iter().map(|(k, v)| (k.to_string(), v.to_string())));
        let canonical_query = canonical_query(&query);
        let canonical = format!(
            "{method}\n{}\n{canonical_query}\n{}\n{signed_headers}\nUNSIGNED-PAYLOAD",
            url.path(),
            headers.iter().map(|(k, v)| format!("{k}:{v}\n")).collect::<String>(),
        );
        let signature = self.signature(&amz_date, &scope, &canonical);
        Ok(format!("{}?{canonical_query}&X-Amz-Signature={signature}", url.as_str().split('?').next().unwrap_or("")))
    }

    /// Request API → Garage, ký bằng header `Authorization`.
    async fn send(&self, method: Method, key: &str, range: Option<&str>) -> AppResult<reqwest::Response> {
        let url = self.object_url(&self.cfg.endpoint, key)?;
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let scope = format!("{}/{}/s3/aws4_request", &amz_date[..8], self.cfg.region);
        let host = host_header(&url);
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical = format!(
            "{method}\n{}\n\nhost:{host}\nx-amz-content-sha256:{EMPTY_SHA256}\nx-amz-date:{amz_date}\n\n{signed_headers}\n{EMPTY_SHA256}",
            url.path()
        );
        let signature = self.signature(&amz_date, &scope, &canonical);
        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.cfg.access_key
        );
        let mut req = self
            .http
            .request(method, url)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", EMPTY_SHA256)
            .header("authorization", auth);
        if let Some(r) = range {
            req = req.header("range", r);
        }
        req.send().await.map_err(|e| AppError::internal(format!("không gọi được Garage: {e}")))
    }

    /// `PUT` có nội dung: ký cả SHA-256 của nội dung và `Content-Type`.
    async fn put(&self, key: &str, bytes: &[u8], content_type: &str) -> AppResult<reqwest::Response> {
        let url = self.object_url(&self.cfg.endpoint, key)?;
        let now = Utc::now();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let scope = format!("{}/{}/s3/aws4_request", &amz_date[..8], self.cfg.region);
        let host = host_header(&url);
        let payload = hex::encode(Sha256::digest(bytes));
        let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";
        let canonical = format!(
            "PUT\n{}\n\ncontent-type:{content_type}\nhost:{host}\nx-amz-content-sha256:{payload}\nx-amz-date:{amz_date}\n\n{signed_headers}\n{payload}",
            url.path()
        );
        let signature = self.signature(&amz_date, &scope, &canonical);
        let auth = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.cfg.access_key
        );
        self.http
            .put(url)
            .header("content-type", content_type)
            .header("x-amz-date", &amz_date)
            .header("x-amz-content-sha256", &payload)
            .header("authorization", auth)
            .body(bytes.to_vec())
            .send()
            .await
            .map_err(|e| AppError::internal(format!("không gọi được Garage: {e}")))
    }

    fn signature(&self, amz_date: &str, scope: &str, canonical_request: &str) -> String {
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );
        let key = signing_key(&self.cfg.secret_key, &amz_date[..8], &self.cfg.region, "s3");
        hex::encode(hmac_sha256(&key, string_to_sign.as_bytes()))
    }
}

/// `host[:port]` đúng như trình duyệt / reqwest gửi (bỏ cổng mặc định).
fn host_header(url: &Url) -> String {
    let host = url.host_str().unwrap_or_default();
    match url.port() {
        Some(p) => format!("{host}:{p}"),
        None => host.to_string(),
    }
}

fn canonical_query(params: &[(String, String)]) -> String {
    let mut encoded: Vec<(String, String)> =
        params.iter().map(|(k, v)| (uri_encode(k, true), uri_encode(v, true))).collect();
    encoded.sort();
    encoded.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("&")
}

/// Mã hóa URI theo SigV4: giữ `A-Z a-z 0-9 - _ . ~`; `/` giữ nguyên trong đường dẫn (`encode_slash = false`).
pub fn uri_encode(input: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b'/' if !encode_slash => out.push('/'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// HMAC-SHA256 (RFC 2104).
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Sha256::new();
    inner.update(block.map(|b| b ^ 0x36));
    inner.update(data);
    let mut outer = Sha256::new();
    outer.update(block.map(|b| b ^ 0x5c));
    outer.update(inner.finalize());
    let mut out = [0u8; 32];
    out.copy_from_slice(&outer.finalize());
    out
}

fn signing_key(secret: &str, date: &str, region: &str, service: &str) -> [u8; 32] {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"aws4_request")
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn hmac_dung_vector_rfc4231() {
        // RFC 4231, test case 2.
        assert_eq!(
            hex::encode(hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        // Khóa dài hơn 64 byte (test case 6).
        assert_eq!(
            hex::encode(hmac_sha256(&[0xaa; 131], b"Test Using Larger Than Block-Size Key - Hash Key First")),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn ma_hoa_uri_theo_sigv4() {
        assert_eq!(uri_encode("content/2026/a b+c~ố.png", false), "content/2026/a%20b%2Bc~%E1%BB%91.png");
        assert_eq!(uri_encode("a/b", true), "a%2Fb");
    }

    fn store() -> S3Store {
        S3Store {
            cfg: StorageConfig {
                endpoint: "http://127.0.0.1:3900".into(),
                public_url: "https://sls.example.com".into(),
                bucket: "files".into(),
                region: "garage".into(),
                // Khóa mẫu trong tài liệu AWS.
                access_key: "AKIAIOSFODNN7EXAMPLE".into(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            },
            http: reqwest::Client::new(),
        }
    }

    #[test]
    fn presigned_url_dung_dinh_dang_va_on_dinh() {
        let s = store();
        let now = Utc.with_ymd_and_hms(2013, 5, 24, 0, 0, 0).unwrap();
        let url =
            s.presign(Method::PUT, "homework/a b.png", Some("image/png"), &[], Duration::from_secs(900), now).unwrap();
        assert!(url.starts_with("https://sls.example.com/files/homework/a%20b.png?X-Amz-Algorithm=AWS4-HMAC-SHA256"));
        assert!(url.contains("X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fgarage%2Fs3%2Faws4_request"));
        assert!(url.contains("X-Amz-SignedHeaders=content-type%3Bhost"));
        assert!(url.contains("X-Amz-Expires=900"));
        let sig = url.rsplit("X-Amz-Signature=").next().unwrap();
        assert_eq!(sig.len(), 64);
        // Cùng đầu vào → cùng chữ ký; đổi Content-Type → chữ ký khác.
        let again =
            s.presign(Method::PUT, "homework/a b.png", Some("image/png"), &[], Duration::from_secs(900), now).unwrap();
        assert_eq!(url, again);
        let other =
            s.presign(Method::PUT, "homework/a b.png", Some("text/html"), &[], Duration::from_secs(900), now).unwrap();
        assert_ne!(url.rsplit('=').next(), other.rsplit('=').next());
    }

    /// Chạy với Garage dev thật: `cargo test -p sls-core --lib storage -- --ignored`
    /// (cần `docker compose -f deploy/docker-compose.dev.yml up -d garage` + `sh deploy/garage/init.sh`).
    #[tokio::test]
    #[ignore]
    async fn garage_dev_upload_xem_xoa_that() {
        let cfg = StorageConfig {
            endpoint: "http://127.0.0.1:3900".into(),
            public_url: "http://127.0.0.1:3900".into(),
            bucket: "files".into(),
            region: "garage".into(),
            access_key: "GK736c732d6465762d6b657930".into(),
            secret_key: "736c732d6465762d7365637265742d6b65792d30313233343536373839616263".into(),
        };
        let storage = Storage::from_config(Some(&cfg));
        let key = format!("test/{}/ảnh thử.png", uuid::Uuid::now_v7());
        let body = b"\x89PNG\r\n\x1a\nnoi-dung-thu".to_vec();
        let http = reqwest::Client::new();

        let put = storage.presign_put(&key, "image/png", Duration::from_secs(60)).unwrap();
        let res = http.put(&put).header("content-type", "image/png").body(body.clone()).send().await.unwrap();
        assert!(res.status().is_success(), "PUT: {} {}", res.status(), res.text().await.unwrap_or_default());
        // Gửi sai Content-Type so với chữ ký → bị từ chối.
        let bad = http.put(&put).header("content-type", "text/html").body("<script>").send().await.unwrap();
        assert_eq!(bad.status(), reqwest::StatusCode::FORBIDDEN);

        let info = storage.head(&key).await.unwrap().expect("có file");
        assert_eq!(info.size, body.len() as i64);
        assert_eq!(storage.read_prefix(&key, 4).await.unwrap(), b"\x89PNG");

        let get = storage.presign_get(&key, "image/png", "ảnh thử.png", true, Duration::from_secs(60)).unwrap();
        let res = http.get(&get).send().await.unwrap();
        assert!(res.status().is_success(), "GET: {}", res.status());
        assert_eq!(res.headers()["content-type"], "image/png");
        assert!(res.headers()["content-disposition"].to_str().unwrap().starts_with("inline; filename*=UTF-8''"));
        assert_eq!(res.bytes().await.unwrap().as_ref(), body.as_slice());

        // API tự ghi / đọc cả file (ký nội dung)
        let key2 = format!("test/{}/api.png", uuid::Uuid::now_v7());
        storage.put(&key2, &body, "image/png").await.unwrap();
        assert_eq!(storage.get(&key2, 1024).await.unwrap(), body);
        assert!(storage.get(&key2, 4).await.is_err());

        storage.delete(&key).await.unwrap();
        storage.delete(&key2).await.unwrap();
        assert!(storage.head(&key).await.unwrap().is_none());
    }

    #[test]
    fn chu_ky_khop_vi_du_presigned_get_cua_aws() {
        // Ví dụ "Query string authentication" trong tài liệu S3 SigV4 (bucket examplebucket, us-east-1).
        let s = S3Store {
            cfg: StorageConfig {
                endpoint: String::new(),
                public_url: "https://examplebucket.s3.amazonaws.com".into(),
                bucket: String::new(),
                region: "us-east-1".into(),
                access_key: "AKIAIOSFODNN7EXAMPLE".into(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".into(),
            },
            http: reqwest::Client::new(),
        };
        let amz_date = "20130524T000000Z";
        let scope = "20130524/us-east-1/s3/aws4_request";
        let canonical = "GET\n/test.txt\nX-Amz-Algorithm=AWS4-HMAC-SHA256&X-Amz-Credential=AKIAIOSFODNN7EXAMPLE%2F20130524%2Fus-east-1%2Fs3%2Faws4_request&X-Amz-Date=20130524T000000Z&X-Amz-Expires=86400&X-Amz-SignedHeaders=host\nhost:examplebucket.s3.amazonaws.com\n\nhost\nUNSIGNED-PAYLOAD";
        assert_eq!(
            s.signature(amz_date, scope, canonical),
            "aeeed9bbccd4d02ee5c0109b86d86835f995330da4c265957d157751f604d404"
        );
    }
}

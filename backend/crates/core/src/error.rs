//! Lỗi thống nhất của API.
//!
//! Mọi lỗi trả về client có dạng:
//! `{ "error": { "code": "VALIDATION_FAILED", "message": "…", "fields": { "email": ["…"] } } }`

use std::{collections::BTreeMap, fmt};

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    /// Mã lỗi ổn định để frontend xử lý (không đổi khi đổi câu chữ).
    pub code: &'static str,
    /// Câu thông báo cho người dùng (tiếng Việt).
    pub message: String,
    pub fields: Option<BTreeMap<String, Vec<String>>>,
    pub retry_after_secs: Option<u64>,
}

impl AppError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into(), fields: None, retry_after_secs: None }
    }

    pub fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub fn unauthorized(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    pub fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", message)
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub fn too_many_requests(retry_after_secs: u64) -> Self {
        let mut e = Self::new(
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMITED",
            format!("Bạn thao tác quá nhiều lần. Vui lòng thử lại sau {retry_after_secs} giây."),
        );
        e.retry_after_secs = Some(retry_after_secs);
        e
    }

    /// Lỗi validate theo từng trường. Tên trường đã ở dạng camelCase như JSON gửi lên.
    pub fn validation(fields: BTreeMap<String, Vec<String>>) -> Self {
        let mut e = Self::new(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION_FAILED", "Dữ liệu chưa hợp lệ.");
        e.fields = Some(fields);
        e
    }

    /// Một trường cụ thể không hợp lệ (dùng trong service).
    pub fn field(field: &str, message: impl Into<String>) -> Self {
        Self::validation(BTreeMap::from([(field.to_string(), vec![message.into()])]))
    }

    /// Lỗi hệ thống: ghi log chi tiết, chỉ trả thông báo chung cho client.
    pub fn internal(source: impl fmt::Display) -> Self {
        tracing::error!(error = %source, "lỗi hệ thống");
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Hệ thống gặp lỗi, vui lòng thử lại.")
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}: {}", self.status.as_u16(), self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::internal(e)
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(errors: validator::ValidationErrors) -> Self {
        let mut fields = BTreeMap::new();
        for (name, errs) in errors.field_errors() {
            let messages = errs
                .iter()
                .map(|e| e.message.as_ref().map(|m| m.to_string()).unwrap_or_else(|| e.code.to_string()))
                .collect();
            fields.insert(snake_to_camel(&name), messages);
        }
        Self::validation(fields)
    }
}

fn snake_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper = false;
    for c in s.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorInner<'a>,
}

#[derive(Serialize)]
struct ErrorInner<'a> {
    code: &'a str,
    message: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<&'a BTreeMap<String, Vec<String>>>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let body =
            ErrorBody { error: ErrorInner { code: self.code, message: &self.message, fields: self.fields.as_ref() } };
        let mut res = (self.status, Json(body)).into_response();
        if let Some(secs) = self.retry_after_secs
            && let Ok(v) = HeaderValue::from_str(&secs.to_string())
        {
            res.headers_mut().insert(header::RETRY_AFTER, v);
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doi_ten_truong_sang_camel_case() {
        assert_eq!(snake_to_camel("full_name"), "fullName");
        assert_eq!(snake_to_camel("new_password_x"), "newPasswordX");
        assert_eq!(snake_to_camel("email"), "email");
    }

    #[tokio::test]
    async fn loi_tra_ve_dung_ma_http_va_json() {
        let res = AppError::conflict("EMAIL_TAKEN", "Email đã được dùng").into_response();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["error"]["code"], "EMAIL_TAKEN");
        assert!(json["error"].get("fields").is_none());
    }

    #[test]
    fn rate_limit_co_header_retry_after() {
        let res = AppError::too_many_requests(42).into_response();
        assert_eq!(res.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(res.headers()[header::RETRY_AFTER], "42");
    }
}

//! `sls-core` — phần dùng chung cho binary `api` và `worker`.
//!
//! - Hạ tầng: cấu hình, kết nối DB, lỗi thống nhất, hàng đợi việc nền, giới hạn tần suất.
//! - `security`: mật khẩu, JWT, token ngẫu nhiên, phân quyền, extractor `Actor`.
//! - `modules`: các module nghiệp vụ, mỗi module có `model` / `dto` / `repository` / `service`.

pub mod config;
pub mod context;
pub mod db;
pub mod error;
pub mod jobs;
pub mod mail;
pub mod modules;
pub mod pagination;
pub mod rate_limit;
pub mod security;
pub mod slug;

pub use context::AppContext;
pub use error::{AppError, AppResult};

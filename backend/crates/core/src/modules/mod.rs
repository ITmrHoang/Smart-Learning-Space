//! Các module nghiệp vụ. Quy tắc: module A chỉ gọi `service` công khai của module B,
//! không gọi thẳng `repository` của B.

pub mod audit;
pub mod auth;
pub mod catalog;
pub mod classes;
pub mod exams;
pub mod lessons;
pub mod questions;
pub mod sso;
pub mod teachers;
pub mod users;

//! Controller: đọc request (DTO đã validate, người dùng hiện tại, IP), gọi service, trả JSON.
//! Không chứa nghiệp vụ. Mỗi file tự khai báo route của mình.

pub mod audit;
pub mod auth;
pub mod catalog;
pub mod health;
pub mod learning;
pub mod sso;
pub mod teachers;
pub mod teaching;
pub mod users;

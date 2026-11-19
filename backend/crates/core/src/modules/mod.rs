//! Các module nghiệp vụ. Quy tắc: module A chỉ gọi `service` công khai của module B,
//! không gọi thẳng `repository` của B.

pub mod assignments;
pub mod audit;
pub mod auth;
pub mod catalog;
pub mod classes;
pub mod discussions;
pub mod exams;
pub mod files;
pub mod flashcards;
pub mod groups;
pub mod guardians;
pub mod lesson_plans;
pub mod lessons;
pub mod live;
pub mod notifications;
pub mod profiles;
pub mod questions;
pub mod sessions;
pub mod snippets;
pub mod sso;
pub mod teachers;
pub mod tutoring;
pub mod users;

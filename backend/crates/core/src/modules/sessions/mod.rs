//! Lịch & buổi học kiểu Google Meet (M14) + nhắc lịch (M15).
//!
//! - Buổi lẻ hoặc lặp hằng tuần (`schedules` nhớ luật, `sessions` là từng buổi, sinh sẵn tối đa 200 buổi).
//! - Mời theo lớp học / nhóm / từng học sinh (`session_invitees`); danh sách người được mời tính lúc xem.
//! - Ai được vào: `invited` (người được mời) · `members` (thành viên lớp học của buổi) · `logged_in` (mọi tài khoản).
//! - Chưa có phòng live (G5): "Vào học" mở `meeting_url` do giáo viên dán (Google Meet / Zoom…) từ 10 phút trước giờ học,
//!   đồng thời ghi điểm danh.
//! - Email mời kèm `.ics` (xem `ics.rs`); dời / hủy gửi lại cùng UID, tăng SEQUENCE.

pub mod dto;
pub mod ics;
pub mod model;
pub mod repository;
pub mod service;
pub mod time;

//! Thư viện bài học (M7).
//!
//! - Bài thuộc giáo viên tạo ra (`owner_id`); chỉ người đó xem / sửa bản nháp.
//! - Bảng `lessons` là **bản nháp**. "Xuất bản" chép bản nháp sang `lesson_revisions` (phiên bản tăng dần)
//!   và trỏ `published_revision_id` tới bản mới. Học sinh luôn đọc bản xuất bản mới nhất, nên sửa bài một lần
//!   thì mọi lớp học dùng bài đó đều có bản mới.
//! - Nội dung: Markdown rút gọn + công thức LaTeX `$…$`; video: link YouTube hoặc link file (mp4 / m3u8).

pub mod dto;
pub mod model;
pub mod repository;
pub mod service;

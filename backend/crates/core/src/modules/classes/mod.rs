//! Lớp học + ghi danh (M8) và lộ trình + tiến độ (M9).
//!
//! - Lớp học thuộc một giáo viên, một môn, một hoặc nhiều lớp (khối), gắn danh mục. Trên giao diện học sinh,
//!   lớp học kiểu **tự học** chính là "khóa học".
//! - Hiển thị: `public` (ai cũng tìm thấy), `unlisted` (chỉ ai có link), `private` (chỉ thành viên).
//!   Ghi danh: `open` (vào ngay), `approval` (giáo viên duyệt), `invite` (chỉ qua mã mời / giáo viên thêm).
//!   Mã mời luôn cho vào thẳng lớp, bất kể chế độ ghi danh.
//! - Lộ trình: `path_steps` là bản nháp; xuất bản lưu ảnh chụp JSON vào `path_versions`; học sinh đọc phiên
//!   bản mới nhất. Tiến độ gắn với ID bước nên đổi thứ tự / sửa không mất tiến độ (xem `path.rs`).

pub mod dto;
pub mod model;
pub mod path;
pub mod repository;
pub mod service;

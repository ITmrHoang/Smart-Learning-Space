//! Ngân hàng câu hỏi (M10).
//!
//! - Câu hỏi thuộc giáo viên tạo ra; gắn môn / lớp / danh mục / mức độ; thư mục chỉ để tự sắp xếp.
//! - 4 dạng: `single` (một đáp án), `multiple` (nhiều đáp án), `true_false` (đúng/sai từng ý, kiểu THPT 2025),
//!   `short` (trả lời ngắn).
//! - Nội dung lưu ở `question_versions.body` (JSON). Mỗi lần sửa tạo phiên bản mới; **ID đáp án giữ nguyên**
//!   giữa các phiên bản, nên chấm theo ID (không theo vị trí A/B/C/D) và chấm lại được khi sửa đáp án.
//! - Import Excel: `import::parse_xlsx` đọc file mẫu → giáo viên xem trước, sửa → lưu hàng loạt.

pub mod dto;
pub mod grading;
pub mod import;
pub mod model;
pub mod repository;
pub mod service;

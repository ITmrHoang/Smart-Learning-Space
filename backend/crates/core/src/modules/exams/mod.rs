//! Đề ôn tập / kiểm tra và lượt làm bài (M11).
//!
//! - Đề thuộc giáo viên tạo ra, dùng được ở nhiều lớp học (là một bước trong lộ trình).
//! - Đề chia phần (I / II / III). Mỗi phần có câu **cố định** và / hoặc **ma trận** (rút ngẫu nhiên N câu theo
//!   danh mục × mức độ × dạng câu × thư mục cho mỗi lượt làm).
//! - Bắt đầu làm bài: server tạo `attempt_items` riêng cho lượt đó (câu đã rút, thứ tự câu đã trộn **trong
//!   từng phần**, thứ tự phương án đã trộn, phiên bản câu hỏi). Chấm theo ID phương án.
//! - Đáp án đúng không gửi xuống trình duyệt trước khi nộp (trừ câu ôn tập vừa bấm "Kiểm tra").
//! - Đồng hồ tính ở server: quá hạn thì lần gọi API kế tiếp tự nộp bài.

pub mod dto;
pub mod model;
pub mod repository;
pub mod service;

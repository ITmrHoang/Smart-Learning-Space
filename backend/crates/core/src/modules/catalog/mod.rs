//! Master data (M4): môn học, lớp (Lớp 1–12 + lớp riêng), danh mục (nhãn trong một môn),
//! mức độ câu hỏi.
//!
//! Lớp riêng / danh mục do giáo viên tự thêm chỉ người đó thấy; admin có thể **đưa lên dùng chung**
//! hoặc **gộp** vào một mục chung có sẵn.

pub mod dto;
pub mod model;
pub mod repository;
pub mod service;

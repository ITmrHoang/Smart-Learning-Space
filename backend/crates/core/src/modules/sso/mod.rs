//! Đăng nhập SSO (M6).
//!
//! Phần 1 (`service`): nhận danh tính **đã được nhà cung cấp xác thực** (`ProviderIdentity`) và quyết định
//! đăng nhập / tạo tài khoản / yêu cầu chứng minh sở hữu tài khoản trước khi liên kết; quản lý liên kết.
//! Phần 2 (`flow` + `oidc`): luồng OIDC thật với Google / Microsoft (chuyển hướng, đổi code, kiểm tra
//! id_token) rồi gọi `service::complete_login`; liên kết thêm danh tính khi đã đăng nhập.
//!
//! Chính sách (analyst.md mục 3, đã chốt 2026-11-05):
//! 1. Danh tính đã liên kết → đăng nhập.
//! 2. Nhà cung cấp chưa xác minh email → từ chối.
//! 3. Email chưa có tài khoản → tạo tài khoản học sinh (không mật khẩu), liên kết, đăng nhập.
//! 4. Email trùng tài khoản **đã xác minh** → phải nhập mật khẩu tài khoản đó hoặc mã OTP gửi về email.
//! 5. Email trùng tài khoản **chưa xác minh** → người đăng nhập SSO là chủ email thật: xóa mật khẩu cũ,
//!    thu hồi mọi phiên, liên kết, đăng nhập (chặn kiểu chiếm tài khoản bằng cách đăng ký trước).

pub mod dto;
pub mod flow;
pub mod model;
pub mod oidc;
pub mod repository;
pub mod service;

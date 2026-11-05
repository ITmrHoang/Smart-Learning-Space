//! Nội dung email. `api` dựng sẵn nội dung, `worker` chỉ việc gửi.

use crate::jobs::Email;

pub fn verification_code(to: &str, full_name: &str, code: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: format!("Mã xác minh Smart Learning Space: {code}"),
        text: format!(
            "Chào {full_name},\n\n\
             Mã xác minh email của bạn là: {code}\n\
             Mã có hiệu lực trong 15 phút. Không chia sẻ mã này cho người khác.\n\n\
             Nếu bạn không đăng ký tài khoản, hãy bỏ qua email này.\n\n\
             Smart Learning Space"
        ),
    }
}

pub fn password_reset(to: &str, full_name: &str, link: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: "Đặt lại mật khẩu Smart Learning Space".to_string(),
        text: format!(
            "Chào {full_name},\n\n\
             Mở link sau để đặt mật khẩu mới (hiệu lực 30 phút, dùng một lần):\n{link}\n\n\
             Nếu bạn không yêu cầu đặt lại mật khẩu, hãy bỏ qua email này; mật khẩu hiện tại vẫn giữ nguyên.\n\n\
             Smart Learning Space"
        ),
    }
}

pub fn teacher_application_approved(to: &str, full_name: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: "Hồ sơ giáo viên đã được duyệt".to_string(),
        text: format!(
            "Chào {full_name},\n\n\
             Hồ sơ đăng ký giáo viên của bạn đã được duyệt. Hãy đăng nhập lại để bắt đầu tạo lớp học.\n\n\
             Smart Learning Space"
        ),
    }
}

pub fn teacher_application_rejected(to: &str, full_name: &str, reason: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: "Kết quả hồ sơ giáo viên".to_string(),
        text: format!(
            "Chào {full_name},\n\n\
             Hồ sơ đăng ký giáo viên của bạn chưa được duyệt.\n\
             Lý do: {reason}\n\n\
             Bạn có thể bổ sung thông tin và gửi lại hồ sơ.\n\n\
             Smart Learning Space"
        ),
    }
}

pub fn sso_link_code(to: &str, full_name: &str, code: &str, provider: &str) -> Email {
    Email {
        to: to.to_string(),
        subject: format!("Mã liên kết tài khoản {provider}: {code}"),
        text: format!(
            "Chào {full_name},\n\n\
             Có yêu cầu liên kết đăng nhập {provider} với tài khoản Smart Learning Space của bạn.\n\
             Mã xác minh là: {code}\n\
             Mã có hiệu lực 15 phút. Nếu không phải bạn, hãy bỏ qua email này; tài khoản vẫn an toàn.\n\n\
             Smart Learning Space"
        ),
    }
}

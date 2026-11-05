//! Phân quyền theo vai trò (RBAC).
//!
//! Danh sách quyền và bảng vai trò → quyền được viết trong code vì hệ thống có 3 vai trò cố định.
//! Quyền theo **từng lớp học** (ai là giáo viên của lớp nào…) kiểm tra trong service của module đó.

use serde::Serialize;

use crate::modules::users::model::UserRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Permission {
    /// Vào khu vực quản trị.
    #[serde(rename = "admin.access")]
    AdminAccess,
    #[serde(rename = "users.read")]
    UsersRead,
    #[serde(rename = "users.manage")]
    UsersManage,
    #[serde(rename = "audit.read")]
    AuditRead,
    /// Thêm / sửa / gộp môn học, lớp, danh mục dùng chung.
    #[serde(rename = "master_data.manage")]
    MasterDataManage,
    /// Giáo viên tự thêm lớp riêng / danh mục (chỉ mình thấy cho tới khi admin đưa lên chung).
    #[serde(rename = "master_data.propose")]
    MasterDataPropose,
    #[serde(rename = "teacher_applications.review")]
    TeacherApplicationsReview,
    /// Gửi hồ sơ đăng ký làm giáo viên.
    #[serde(rename = "teacher_applications.submit")]
    TeacherApplicationsSubmit,
    /// Soạn bài học, lớp học, câu hỏi, đề (giáo viên; admin cũng có để thử).
    #[serde(rename = "teaching.manage")]
    TeachingManage,
}

impl Permission {
    pub const ALL: &'static [Permission] = &[
        Permission::AdminAccess,
        Permission::UsersRead,
        Permission::UsersManage,
        Permission::AuditRead,
        Permission::MasterDataManage,
        Permission::MasterDataPropose,
        Permission::TeacherApplicationsReview,
        Permission::TeacherApplicationsSubmit,
        Permission::TeachingManage,
    ];
}

const ADMIN: &[Permission] = &[
    Permission::AdminAccess,
    Permission::UsersRead,
    Permission::UsersManage,
    Permission::AuditRead,
    Permission::MasterDataManage,
    Permission::MasterDataPropose,
    Permission::TeacherApplicationsReview,
    Permission::TeachingManage,
];
const TEACHER: &[Permission] = &[Permission::MasterDataPropose, Permission::TeachingManage];
const STUDENT: &[Permission] = &[Permission::TeacherApplicationsSubmit];

/// Bảng vai trò → quyền.
pub fn permissions_for(role: UserRole) -> &'static [Permission] {
    match role {
        UserRole::Admin => ADMIN,
        UserRole::Teacher => TEACHER,
        UserRole::Student => STUDENT,
    }
}

pub fn has_permission(role: UserRole, permission: Permission) -> bool {
    permissions_for(role).contains(&permission)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_co_moi_quyen_quan_tri() {
        for p in ADMIN {
            assert!(has_permission(UserRole::Admin, *p));
        }
        // Admin không cần "gửi hồ sơ giáo viên".
        assert!(!has_permission(UserRole::Admin, Permission::TeacherApplicationsSubmit));
    }

    #[test]
    fn giao_vien_va_hoc_sinh_khong_vao_duoc_quan_tri() {
        for role in [UserRole::Teacher, UserRole::Student] {
            assert!(!has_permission(role, Permission::AdminAccess));
            assert!(!has_permission(role, Permission::UsersManage));
            assert!(!has_permission(role, Permission::MasterDataManage));
        }
        assert!(has_permission(UserRole::Teacher, Permission::MasterDataPropose));
        assert!(!has_permission(UserRole::Student, Permission::MasterDataPropose));
        assert!(has_permission(UserRole::Student, Permission::TeacherApplicationsSubmit));
    }

    #[test]
    fn ten_quyen_dang_chuoi_cho_frontend() {
        assert_eq!(serde_json::to_value(Permission::AdminAccess).unwrap(), "admin.access");
        assert_eq!(Permission::ALL.len(), 9);
        assert!(has_permission(UserRole::Teacher, Permission::TeachingManage));
        assert!(!has_permission(UserRole::Student, Permission::TeachingManage));
    }
}

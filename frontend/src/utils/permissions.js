/** Tên quyền — khớp `Permission` ở backend (core/security/permissions.rs). */
export const P = Object.freeze({
  ADMIN: 'admin.access',
  USERS_READ: 'users.read',
  USERS_MANAGE: 'users.manage',
  AUDIT_READ: 'audit.read',
  MASTER_DATA_MANAGE: 'master_data.manage',
  MASTER_DATA_PROPOSE: 'master_data.propose',
  TEACHER_REVIEW: 'teacher_applications.review',
  TEACHER_SUBMIT: 'teacher_applications.submit',
});

export const ROLE_LABELS = { admin: 'Quản trị viên', teacher: 'Giáo viên', student: 'Học sinh' };
export const ROLE_COLORS = { admin: 'danger', teacher: 'primary', student: 'success' };
export const STATUS_LABELS = { pending_verification: 'Chờ xác minh', active: 'Hoạt động', locked: 'Đã khóa' };
export const STATUS_COLORS = { pending_verification: 'warning', active: 'success', locked: 'danger' };

/** Menu bên trái. Mục có `permission` chỉ hiện khi người dùng có quyền đó. */
export const MENU = [
  {
    title: null,
    items: [
      { to: '/', label: 'Trang chủ' },
      { to: '/ho-so', label: 'Hồ sơ của tôi' },
      { to: '/dang-ky-giao-vien', label: 'Đăng ký làm giáo viên', permission: P.TEACHER_SUBMIT },
    ],
  },
  {
    title: 'Quản trị',
    items: [
      { to: '/admin/nguoi-dung', label: 'Người dùng', permission: P.USERS_READ },
      { to: '/admin/ho-so-giao-vien', label: 'Hồ sơ giáo viên', permission: P.TEACHER_REVIEW },
      { to: '/admin/mon-hoc', label: 'Môn học', permission: P.MASTER_DATA_MANAGE },
      { to: '/admin/lop', label: 'Lớp', permission: P.MASTER_DATA_MANAGE },
      { to: '/admin/danh-muc', label: 'Danh mục', permission: P.MASTER_DATA_MANAGE },
      { to: '/admin/muc-giao-vien-them', label: 'Mục giáo viên thêm', permission: P.MASTER_DATA_MANAGE },
      { to: '/admin/nhat-ky', label: 'Nhật ký thao tác', permission: P.AUDIT_READ },
    ],
  },
];

/** Lọc menu theo quyền; bỏ nhóm không còn mục nào. */
export function visibleMenu(permissions) {
  const has = (p) => !p || permissions.includes(p);
  return MENU.map((group) => ({ ...group, items: group.items.filter((item) => has(item.permission)) })).filter(
    (group) => group.items.length > 0,
  );
}

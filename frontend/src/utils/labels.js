/** Nhãn tiếng Việt cho các giá trị từ API. */

export const APP_STATUS_LABELS = { pending: 'Chờ duyệt', approved: 'Đã duyệt', rejected: 'Bị từ chối' };
export const APP_STATUS_COLORS = { pending: 'warning', approved: 'success', rejected: 'danger' };

export const AUDIT_ACTION_LABELS = {
  'user.create': 'Tạo tài khoản',
  'user.update': 'Sửa tài khoản',
  'user.lock': 'Khóa tài khoản',
  'user.unlock': 'Mở khóa tài khoản',
  'user.password_reset_sent': 'Gửi link đặt lại mật khẩu',
  'subject.create': 'Thêm môn học',
  'subject.update': 'Sửa môn học',
  'grade.create': 'Thêm lớp',
  'grade.update': 'Sửa lớp',
  'grade.promote': 'Đưa lớp lên dùng chung',
  'grade.merge': 'Gộp lớp',
  'tag.create': 'Thêm danh mục',
  'tag.update': 'Sửa danh mục',
  'tag.promote': 'Đưa danh mục lên dùng chung',
  'tag.merge': 'Gộp danh mục',
  'teacher_application.approve': 'Duyệt hồ sơ giáo viên',
  'teacher_application.reject': 'Từ chối hồ sơ giáo viên',
};

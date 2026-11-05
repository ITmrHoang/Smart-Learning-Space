/** Các hàm gọi API theo module (khớp backend `controllers/*`). */
import { http } from './http';

export const authApi = {
  register: (data) => http.post('/auth/register', data),
  verifyEmail: (email, code) => http.post('/auth/verify-email', { email, code }),
  resendVerification: (email) => http.post('/auth/resend-verification', { email }),
  login: (login, password) => http.post('/auth/login', { login, password }),
  logout: () => http.post('/auth/logout'),
  logoutAll: () => http.post('/auth/logout-all'),
  forgotPassword: (email) => http.post('/auth/forgot-password', { email }),
  resetPassword: (token, newPassword) => http.post('/auth/reset-password', { token, newPassword }),
  changePassword: (currentPassword, newPassword) => http.post('/auth/change-password', { currentPassword, newPassword }),
  /** Tài khoản tạo từ SSO đặt mật khẩu lần đầu. */
  setPassword: (newPassword) => http.post('/auth/set-password', { newPassword }),
  me: () => http.get('/auth/me'),
};

export const ssoApi = {
  /** Các nhà cung cấp đã bật: { providers: ['google', 'microsoft'] } */
  providers: () => http.get('/auth/sso/providers'),
  /** mode: 'login' | 'link' (liên kết thêm khi đã đăng nhập). Trả { authorizeUrl }. */
  start: (provider, { mode = 'login', next } = {}) => http.post(`/auth/sso/${provider}/start`, { mode, next }),
  sendLinkCode: (token) => http.post('/auth/sso/link/send-code', { token }),
  /** Xác nhận liên kết bằng { token, password } hoặc { token, code } → phiên đăng nhập. */
  confirmLink: (data) => http.post('/auth/sso/link', data),
  identities: () => http.get('/auth/identities'),
  unlink: (provider) => http.del(`/auth/identities/${provider}`),
};

export const catalogApi = {
  masterData: () => http.get('/master-data'),
  tags: (subjectId) => http.get(`/subjects/${subjectId}/tags`),
  /** Giáo viên: thêm lớp riêng; admin: thêm lớp dùng chung. */
  createGrade: (data) => http.post('/grades', data),
  createTag: (subjectId, data) => http.post(`/subjects/${subjectId}/tags`, data),
};

export const teacherApi = {
  submit: (data) => http.post('/teacher-applications', data),
  mine: () => http.get('/teacher-applications/me'),
};

export const adminApi = {
  users: (query) => http.get('/admin/users', query),
  user: (id) => http.get(`/admin/users/${id}`),
  createUser: (data) => http.post('/admin/users', data),
  updateUser: (id, data) => http.patch(`/admin/users/${id}`, data),
  lockUser: (id) => http.post(`/admin/users/${id}/lock`),
  unlockUser: (id) => http.post(`/admin/users/${id}/unlock`),
  sendPasswordReset: (id) => http.post(`/admin/users/${id}/send-password-reset`),
  userIdentities: (id) => http.get(`/admin/users/${id}/identities`),
  unlinkIdentity: (id, provider) => http.del(`/admin/users/${id}/identities/${provider}`),

  auditLogs: (query) => http.get('/admin/audit-logs', query),

  subjects: () => http.get('/admin/subjects'),
  createSubject: (data) => http.post('/admin/subjects', data),
  updateSubject: (id, data) => http.patch(`/admin/subjects/${id}`, data),
  grades: () => http.get('/admin/grades'),
  updateGrade: (id, data) => http.patch(`/admin/grades/${id}`, data),
  promoteGrade: (id) => http.post(`/admin/grades/${id}/promote`),
  mergeGrade: (id, intoId) => http.post(`/admin/grades/${id}/merge`, { intoId }),
  tags: (subjectId) => http.get(`/admin/subjects/${subjectId}/tags`),
  updateTag: (id, data) => http.patch(`/admin/tags/${id}`, data),
  promoteTag: (id) => http.post(`/admin/tags/${id}/promote`),
  mergeTag: (id, intoId) => http.post(`/admin/tags/${id}/merge`, { intoId }),
  pendingMasterData: () => http.get('/admin/master-data/pending'),

  teacherApplications: (query) => http.get('/admin/teacher-applications', query),
  approveApplication: (id) => http.post(`/admin/teacher-applications/${id}/approve`),
  rejectApplication: (id, reason) => http.post(`/admin/teacher-applications/${id}/reject`, { reason }),
};

export const systemApi = {
  health: () => http.get('/health'),
};

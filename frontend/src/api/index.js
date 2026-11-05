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
  me: () => http.get('/auth/me'),
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

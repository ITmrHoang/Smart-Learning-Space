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

/** Giáo viên soạn nội dung (`/api/teaching/*`). */
export const teachingApi = {
  lessons: (query) => http.get('/teaching/lessons', query),
  lesson: (id) => http.get(`/teaching/lessons/${id}`),
  createLesson: (data) => http.post('/teaching/lessons', data),
  updateLesson: (id, data) => http.put(`/teaching/lessons/${id}`, data),
  archiveLesson: (id) => http.del(`/teaching/lessons/${id}`),
  publishLesson: (id) => http.post(`/teaching/lessons/${id}/publish`),
  lessonRevisions: (id) => http.get(`/teaching/lessons/${id}/revisions`),

  classes: (query) => http.get('/teaching/classes', query),
  class: (id) => http.get(`/teaching/classes/${id}`),
  createClass: (data) => http.post('/teaching/classes', data),
  updateClass: (id, data) => http.put(`/teaching/classes/${id}`, data),
  archiveClass: (id) => http.post(`/teaching/classes/${id}/archive`),
  restoreClass: (id) => http.post(`/teaching/classes/${id}/restore`),
  copyClass: (id, title) => http.post(`/teaching/classes/${id}/copy`, { title }),
  regenerateCode: (id) => http.post(`/teaching/classes/${id}/invite-code`),
  members: (id, status) => http.get(`/teaching/classes/${id}/members`, { status }),
  addMember: (id, login) => http.post(`/teaching/classes/${id}/members`, { login }),
  /** action: approve | reject | remove */
  changeMember: (id, userId, action) => http.post(`/teaching/classes/${id}/members/${userId}/${action}`),
  path: (id) => http.get(`/teaching/classes/${id}/path`),
  savePath: (id, steps) => http.put(`/teaching/classes/${id}/path`, { steps }),
  publishPath: (id) => http.post(`/teaching/classes/${id}/path/publish`),
  progress: (id) => http.get(`/teaching/classes/${id}/progress`),

  questions: (query) => http.get('/teaching/questions', query),
  question: (id) => http.get(`/teaching/questions/${id}`),
  createQuestion: (data) => http.post('/teaching/questions', data),
  updateQuestion: (id, data) => http.put(`/teaching/questions/${id}`, data),
  archiveQuestion: (id) => http.del(`/teaching/questions/${id}`),
  regradeQuestion: (id) => http.post(`/teaching/questions/${id}/regrade`),
  bulkCreateQuestions: (items) => http.post('/teaching/questions/bulk', { items }),
  importTemplate: () => http.blob('/teaching/questions/import/template'),
  importPreview: (subjectId, file) => http.upload('/teaching/questions/import/preview', file, { subjectId }),
  folders: () => http.get('/teaching/question-folders'),
  createFolder: (data) => http.post('/teaching/question-folders', data),
  updateFolder: (id, data) => http.put(`/teaching/question-folders/${id}`, data),
  deleteFolder: (id) => http.del(`/teaching/question-folders/${id}`),

  exams: (query) => http.get('/teaching/exams', query),
  exam: (id) => http.get(`/teaching/exams/${id}`),
  createExam: (data) => http.post('/teaching/exams', data),
  updateExam: (id, data) => http.put(`/teaching/exams/${id}`, data),
  archiveExam: (id) => http.del(`/teaching/exams/${id}`),
  previewExam: (id) => http.post(`/teaching/exams/${id}/preview`),
  examAttempts: (id, classId) => http.get(`/teaching/exams/${id}/attempts`, { classId }),
};

/** Học sinh: lớp học, lộ trình. */
export const learnApi = {
  catalog: (query) => http.get('/classes', query),
  detail: (id) => http.get(`/classes/${id}`),
  enroll: (id) => http.post(`/classes/${id}/enroll`),
  join: (code) => http.post('/classes/join', { code }),
  leave: (id) => http.post(`/classes/${id}/leave`),
  myClasses: () => http.get('/my/classes'),
  learn: (id) => http.get(`/classes/${id}/learn`),
  stepLesson: (id, stepId) => http.get(`/classes/${id}/steps/${stepId}/lesson`),
  completeStep: (id, stepId) => http.post(`/classes/${id}/steps/${stepId}/complete`),
  startExam: (id, stepId) => http.post(`/classes/${id}/steps/${stepId}/attempts`),
  stepAttempts: (id, stepId) => http.get(`/classes/${id}/steps/${stepId}/attempts`),
};

/** Lượt làm bài. */
export const attemptApi = {
  get: (id) => http.get(`/attempts/${id}`),
  /** answer: { selected: [optionId] } | { truth: { optionId: bool } } | { text } */
  save: (id, position, answer) => http.put(`/attempts/${id}/items/${position}`, answer),
  check: (id, position) => http.post(`/attempts/${id}/items/${position}/check`),
  submit: (id) => http.post(`/attempts/${id}/submit`),
};

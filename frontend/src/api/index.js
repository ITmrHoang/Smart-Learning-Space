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

  // Nhóm học sinh (M13)
  students: (query) => http.get('/teaching/students', query),
  groups: () => http.get('/teaching/groups'),
  group: (id) => http.get(`/teaching/groups/${id}`),
  createGroup: (data) => http.post('/teaching/groups', data),
  updateGroup: (id, data) => http.put(`/teaching/groups/${id}`, data),
  deleteGroup: (id) => http.del(`/teaching/groups/${id}`),

  // Lịch & buổi học (M14) — chi tiết buổi dùng sessionApi.detail
  sessions: (query) => http.get('/teaching/sessions', query),
  createSession: (data) => http.post('/teaching/sessions', data),
  /** data.scope: 'this' | 'following' */
  updateSession: (id, data) => http.put(`/teaching/sessions/${id}`, data),
  cancelSession: (id, scope = 'this') => http.post(`/teaching/sessions/${id}/cancel`, { scope }),
  setAttendance: (id, userId, attended) => http.put(`/teaching/sessions/${id}/attendance`, { userId, attended }),

  // Báo cáo lớp học (M17) + xử lý báo vi phạm (M18) + gửi báo cáo phụ huynh (M20)
  classReport: (id) => http.get(`/teaching/classes/${id}/report`),
  classReportXlsx: (id) => http.blob(`/teaching/classes/${id}/report.xlsx`),
  contentReports: (id, status) => http.get(`/teaching/classes/${id}/content-reports`, { status }),
  resolveReport: (id, action) => http.post(`/teaching/content-reports/${id}/resolve`, { action }),
  sendGuardianReport: (classId, userId, note) =>
    http.post(`/teaching/classes/${classId}/students/${userId}/guardian-report`, { note }),

  // Thẻ ghi nhớ (M19)
  decks: (query) => http.get('/teaching/decks', query),
  deck: (id) => http.get(`/teaching/decks/${id}`),
  createDeck: (data) => http.post('/teaching/decks', data),
  updateDeck: (id, data) => http.put(`/teaching/decks/${id}`, data),
  archiveDeck: (id) => http.del(`/teaching/decks/${id}`),
  deckFromQuestions: (id, questionIds) => http.post(`/teaching/decks/${id}/from-questions`, { questionIds }),

  // Bài tập về nhà (M22)
  assignments: (query) => http.get('/teaching/assignments', query),
  assignment: (id) => http.get(`/teaching/assignments/${id}`),
  createAssignment: (data) => http.post('/teaching/assignments', data),
  updateAssignment: (id, data) => http.put(`/teaching/assignments/${id}`, data),
  archiveAssignment: (id) => http.del(`/teaching/assignments/${id}`),
  classHomework: (id) => http.get(`/teaching/classes/${id}/homework`),
  stepSubmissions: (id, stepId) => http.get(`/teaching/classes/${id}/steps/${stepId}/submissions`),
  submission: (id) => http.get(`/teaching/submissions/${id}`),
  /** data: { action: grade | return, score, feedback, annotations: [{ position, fileId }] } */
  gradeSubmission: (id, data) => http.post(`/teaching/submissions/${id}/grade`, data),

  // Học kèm 1-1 (M23)
  tutoringOffers: () => http.get('/teaching/tutoring/offers'),
  createOffer: (data) => http.post('/teaching/tutoring/offers', data),
  updateOffer: (id, data) => http.put(`/teaching/tutoring/offers/${id}`, data),
  /** status: pending | upcoming | all */
  bookings: (status) => http.get('/teaching/tutoring/bookings', { status }),
  approveBooking: (id) => http.post(`/teaching/tutoring/bookings/${id}/approve`),
  declineBooking: (id, reason) => http.post(`/teaching/tutoring/bookings/${id}/decline`, { reason }),
  cancelBooking: (id, reason) => http.post(`/teaching/tutoring/bookings/${id}/cancel`, { reason }),

  // Hồ sơ giáo viên công khai (M25)
  profile: () => http.get('/teaching/profile'),
  updateProfile: (data) => http.put('/teaching/profile', data),
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
  stepDeck: (id, stepId) => http.get(`/classes/${id}/steps/${stepId}/deck`),
  /** rating: again | hard | good | easy */
  review: (id, stepId, cardId, rating) => http.post(`/classes/${id}/steps/${stepId}/reviews`, { cardId, rating }),
  myReviews: () => http.get('/my/reviews'),
  stepHomework: (id, stepId) => http.get(`/classes/${id}/steps/${stepId}/homework`),
  submitHomework: (id, stepId, data) => http.put(`/classes/${id}/steps/${stepId}/homework`, data),
};

/** Lượt làm bài. */
export const attemptApi = {
  get: (id) => http.get(`/attempts/${id}`),
  /** answer: { selected: [optionId] } | { truth: { optionId: bool } } | { text } */
  save: (id, position, answer) => http.put(`/attempts/${id}/items/${position}`, answer),
  check: (id, position) => http.post(`/attempts/${id}/items/${position}/check`),
  submit: (id) => http.post(`/attempts/${id}/submit`),
};

/** Buổi học: lịch của tôi, chi tiết (cả giáo viên), trả lời tham gia, vào học. */
export const sessionApi = {
  /** query: { from, to, classId } — ISO, tối đa 93 ngày */
  mine: (query) => http.get('/my/sessions', query),
  detail: (id) => http.get(`/sessions/${id}`),
  /** response: 'yes' | 'no' */
  rsvp: (id, response) => http.post(`/sessions/${id}/rsvp`, { response }),
  join: (id) => http.post(`/sessions/${id}/join`),
};

/** Thông báo trong app (chuông). */
export const notificationApi = {
  list: (query) => http.get('/notifications', query),
  unreadCount: () => http.get('/notifications/unread-count'),
  read: (id) => http.post(`/notifications/${id}/read`),
  readAll: () => http.post('/notifications/read-all'),
};

/** Hỏi đáp theo bài (M18). */
export const qaApi = {
  /** query: { stepId, status: all | open | answered | mine, q, page, pageSize } */
  threads: (classId, query) => http.get(`/classes/${classId}/threads`, query),
  counts: (classId) => http.get(`/classes/${classId}/qa-counts`),
  create: (classId, data) => http.post(`/classes/${classId}/threads`, data),
  get: (id) => http.get(`/threads/${id}`),
  update: (id, data) => http.put(`/threads/${id}`, data),
  remove: (id) => http.del(`/threads/${id}`),
  reply: (id, body) => http.post(`/threads/${id}/posts`, { body }),
  /** action: pin | unpin | hide | unhide */
  moderateThread: (id, action) => http.post(`/threads/${id}/moderate`, { action }),
  updatePost: (id, body) => http.put(`/posts/${id}`, { body }),
  removePost: (id) => http.del(`/posts/${id}`),
  /** action: accept | unaccept | hide | unhide */
  moderatePost: (id, action) => http.post(`/posts/${id}/moderate`, { action }),
  report: (targetType, targetId, reason) => http.post('/content-reports', { targetType, targetId, reason }),
};

/** Phụ huynh (M20): học sinh khai + trang phụ huynh bấm link trong email (không cần đăng nhập). */
export const guardianApi = {
  mine: () => http.get('/my/guardians'),
  add: (data) => http.post('/my/guardians', data),
  update: (id, data) => http.put(`/my/guardians/${id}`, data),
  remove: (id) => http.del(`/my/guardians/${id}`),
  resend: (id) => http.post(`/my/guardians/${id}/resend`),
  consentInfo: (token) => http.get('/guardians/consent', { token }),
  respond: (token, accept) => http.post('/guardians/consent', { token, accept }),
  subscription: (token) => http.get('/guardians/subscription', { token }),
  unsubscribe: (token) => http.post('/guardians/unsubscribe', { token }),
  subscribe: (token) => http.post('/guardians/subscribe', { token }),
};

/** File (M21): xin link upload → trình duyệt PUT thẳng lên Garage → báo xong. Dùng `utils/upload.js`. */
export const fileApi = {
  /** data: { purpose: content | homework, name, contentType, size } */
  ticket: (data) => http.post('/files', data),
  complete: (id) => http.post(`/files/${id}/complete`),
};

/** Học kèm 1-1 (M23) — phía học sinh. */
export const tutoringApi = {
  offers: () => http.get('/tutoring/offers'),
  offer: (id) => http.get(`/tutoring/offers/${id}`),
  slots: (id) => http.get(`/tutoring/offers/${id}/slots`),
  book: (id, startsAt, message) => http.post(`/tutoring/offers/${id}/bookings`, { startsAt, message }),
  /** status: pending | upcoming | all */
  mine: (status) => http.get('/my/bookings', { status }),
  cancel: (id, reason) => http.post(`/my/bookings/${id}/cancel`, { reason }),
};

/** Hồ sơ giáo viên công khai (M25), xem được cả khi chưa đăng nhập. */
export const publicApi = {
  /** key: username hoặc id */
  teacher: (key) => http.get(`/public/teachers/${encodeURIComponent(key)}`),
};

/** Phòng học trực tuyến (đợt 6, LiveKit). */
export const liveApi = {
  status: () => http.get('/live/status'),
  /** → { url, token, room, identity, name, role, session } */
  join: (id) => http.post(`/sessions/${id}/live/join`),
  /** data: { identity, microphone, camera, screen } — giáo viên cho / thu quyền phát */
  media: (id, data) => http.post(`/sessions/${id}/live/media`, data),
  remove: (id, identity) => http.post(`/sessions/${id}/live/remove`, { identity }),
  end: (id) => http.post(`/sessions/${id}/live/end`),
  turnStart: (id, identity) => http.post(`/sessions/${id}/live/board-turns/start`, { identity }),
  turnEnd: (id, identity) => http.post(`/sessions/${id}/live/board-turns/end`, { identity }),
  guestLink: (id) => http.post(`/sessions/${id}/live/guest-link`),
  guests: (id) => http.get(`/sessions/${id}/live/guests`),
  decide: (id, guestId, admit) => http.post(`/sessions/${id}/live/guests/${guestId}/decide`, { admit }),
  decideAll: (id, admit) => http.post(`/sessions/${id}/live/guests/decide-all`, { admit }),
  savePoll: (id, data) => http.post(`/sessions/${id}/live/polls`, data),
  saveBoard: (id, data) => http.put(`/sessions/${id}/live/board`, data),
  boardData: (id) => http.get(`/sessions/${id}/live/board`),
  records: (id) => http.get(`/sessions/${id}/live/records`),
  // Khách (không cần đăng nhập)
  guestJoin: (id, key, name) => http.post(`/live/guest/${id}/join`, { key, name }),
  guestState: (guestId, token) => http.get(`/live/guest/state/${guestId}`, { token }),
};

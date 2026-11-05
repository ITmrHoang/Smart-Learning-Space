import { createRouter, createWebHistory } from 'vue-router';

import { useAuthStore } from '../stores/auth';
import { resolveGuard } from './guard';
import { P } from '../utils/permissions';

const AuthLayout = () => import('../layouts/AuthLayout.vue');
const AppLayout = () => import('../layouts/AppLayout.vue');

/**
 * meta:
 * - guest: chỉ dành cho người chưa đăng nhập (đăng nhập rồi thì về trang chủ)
 * - auth: phải đăng nhập
 * - permission: phải có quyền này (thiếu → trang 403)
 */
export const routes = [
  // Nhóm AppLayout đặt trước: đường dẫn '/' phải khớp trang chủ, không khớp nhóm AuthLayout.
  {
    path: '/',
    component: AppLayout,
    meta: { auth: true },
    children: [
      { path: '', name: 'home', component: () => import('../pages/HomePage.vue') },
      { path: 'ho-so', name: 'profile', component: () => import('../pages/ProfilePage.vue') },
      {
        path: 'dang-ky-giao-vien',
        name: 'apply-teacher',
        component: () => import('../pages/teacher/ApplyTeacherPage.vue'),
        meta: { permission: P.TEACHER_SUBMIT },
      },
      {
        path: 'admin/nguoi-dung',
        name: 'admin-users',
        component: () => import('../pages/admin/UsersPage.vue'),
        meta: { permission: P.USERS_READ },
      },
      {
        path: 'admin/ho-so-giao-vien',
        name: 'admin-teacher-applications',
        component: () => import('../pages/admin/TeacherApplicationsPage.vue'),
        meta: { permission: P.TEACHER_REVIEW },
      },
      {
        path: 'admin/mon-hoc',
        name: 'admin-subjects',
        component: () => import('../pages/admin/SubjectsPage.vue'),
        meta: { permission: P.MASTER_DATA_MANAGE },
      },
      {
        path: 'admin/lop',
        name: 'admin-grades',
        component: () => import('../pages/admin/GradesPage.vue'),
        meta: { permission: P.MASTER_DATA_MANAGE },
      },
      {
        path: 'admin/danh-muc',
        name: 'admin-tags',
        component: () => import('../pages/admin/TagsPage.vue'),
        meta: { permission: P.MASTER_DATA_MANAGE },
      },
      {
        path: 'admin/muc-giao-vien-them',
        name: 'admin-pending',
        component: () => import('../pages/admin/PendingMasterDataPage.vue'),
        meta: { permission: P.MASTER_DATA_MANAGE },
      },
      {
        path: 'admin/nhat-ky',
        name: 'admin-audit',
        component: () => import('../pages/admin/AuditLogsPage.vue'),
        meta: { permission: P.AUDIT_READ },
      },
      // Học sinh (mọi tài khoản đã đăng nhập)
      { path: 'lop-hoc-cua-toi', name: 'my-classes', component: () => import('../pages/learning/MyClassesPage.vue') },
      { path: 'kham-pha', name: 'explore', component: () => import('../pages/learning/ExploreClassesPage.vue') },
      { path: 'lop-hoc/:id', name: 'class', component: () => import('../pages/learning/ClassPage.vue') },
      { path: 'lop-hoc/:id/bai/:stepId', name: 'lesson-view', component: () => import('../pages/learning/LessonViewPage.vue') },
      { path: 'bai-lam/:id', name: 'attempt', component: () => import('../pages/learning/AttemptPage.vue') },
      // Giáo viên soạn nội dung
      ...[
        ['day-hoc/lop-hoc', 'teach-classes', () => import('../pages/teaching/ClassesPage.vue')],
        ['day-hoc/lop-hoc/:id', 'teach-class', () => import('../pages/teaching/ClassManagePage.vue')],
        ['day-hoc/bai-hoc', 'teach-lessons', () => import('../pages/teaching/LessonsPage.vue')],
        ['day-hoc/bai-hoc/:id', 'teach-lesson', () => import('../pages/teaching/LessonEditorPage.vue')],
        ['day-hoc/cau-hoi', 'teach-questions', () => import('../pages/teaching/QuestionBankPage.vue')],
        ['day-hoc/de', 'teach-exams', () => import('../pages/teaching/ExamsPage.vue')],
        ['day-hoc/de/:id', 'teach-exam', () => import('../pages/teaching/ExamEditorPage.vue')],
      ].map(([path, name, component]) => ({ path, name, component, meta: { permission: P.TEACHING_MANAGE } })),
      { path: 'khong-co-quyen', name: 'forbidden', component: () => import('../pages/ForbiddenPage.vue') },
    ],
  },
  {
    path: '/',
    component: AuthLayout,
    meta: { guest: true },
    children: [
      { path: 'dang-nhap', name: 'login', component: () => import('../pages/auth/LoginPage.vue') },
      { path: 'dang-ky', name: 'register', component: () => import('../pages/auth/RegisterPage.vue') },
      { path: 'xac-minh-email', name: 'verify-email', component: () => import('../pages/auth/VerifyEmailPage.vue') },
      { path: 'quen-mat-khau', name: 'forgot-password', component: () => import('../pages/auth/ForgotPasswordPage.vue') },
    ],
  },
  {
    // Link trong email: mở được cả khi đang đăng nhập.
    path: '/dat-lai-mat-khau',
    component: AuthLayout,
    children: [{ path: '', name: 'reset-password', component: () => import('../pages/auth/ResetPasswordPage.vue') }],
  },
  {
    // Google / Microsoft chuyển về sau khi đăng nhập (backend `/api/auth/sso/{provider}/callback`).
    path: '/sso',
    component: AuthLayout,
    children: [
      { path: 'xong', name: 'sso-done', component: () => import('../pages/auth/SsoDonePage.vue') },
      { path: 'lien-ket', name: 'sso-link', component: () => import('../pages/auth/SsoLinkPage.vue') },
    ],
  },
  { path: '/trang-thai', name: 'status', component: () => import('../pages/StatusPage.vue') },
  { path: '/:pathMatch(.*)*', name: 'not-found', component: () => import('../pages/NotFoundPage.vue') },
];

export const router = createRouter({
  history: createWebHistory(),
  routes,
  scrollBehavior: () => ({ top: 0 }),
});

router.beforeEach((to) => resolveGuard(to, useAuthStore()));

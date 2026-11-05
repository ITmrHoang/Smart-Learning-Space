import { describe, expect, it } from 'vitest';

import { resolveGuard } from '../router/guard';
import { P, visibleMenu } from './permissions';

const labels = (menu) => menu.flatMap((g) => g.items.map((i) => i.label));

describe('menu theo quyền', () => {
  it('học sinh không thấy nhóm Quản trị', () => {
    const menu = visibleMenu([P.TEACHER_SUBMIT]);
    expect(menu).toHaveLength(1);
    expect(labels(menu)).toContain('Đăng ký làm giáo viên');
  });

  it('giáo viên không thấy mục đăng ký giáo viên', () => {
    expect(labels(visibleMenu([P.MASTER_DATA_PROPOSE]))).not.toContain('Đăng ký làm giáo viên');
  });

  it('admin thấy đủ mục quản trị', () => {
    const menu = visibleMenu([P.ADMIN, P.USERS_READ, P.AUDIT_READ, P.MASTER_DATA_MANAGE, P.TEACHER_REVIEW]);
    expect(menu.map((g) => g.title)).toEqual([null, 'Quản trị']);
    expect(labels(menu)).toEqual(
      expect.arrayContaining(['Người dùng', 'Hồ sơ giáo viên', 'Môn học', 'Lớp', 'Danh mục', 'Nhật ký thao tác']),
    );
  });
});

describe('guard của router', () => {
  const route = (path, ...metas) => ({ fullPath: path, matched: metas.map((meta) => ({ meta })) });
  const auth = (loggedIn, permissions = []) => ({ isLoggedIn: loggedIn, can: (p) => permissions.includes(p) });

  it('chưa đăng nhập vào trang cần đăng nhập → về trang đăng nhập kèm redirect', () => {
    expect(resolveGuard(route('/ho-so', { auth: true }, {}), auth(false))).toEqual({
      name: 'login',
      query: { redirect: '/ho-so' },
    });
  });

  it('đã đăng nhập vào trang đăng nhập → về trang chủ', () => {
    expect(resolveGuard(route('/dang-nhap', { guest: true }, {}), auth(true))).toEqual({ name: 'home' });
  });

  it('thiếu quyền → trang 403', () => {
    const to = route('/admin/nguoi-dung', { auth: true }, { permission: P.USERS_READ });
    expect(resolveGuard(to, auth(true, [P.TEACHER_SUBMIT]))).toEqual({ name: 'forbidden' });
    expect(resolveGuard(to, auth(true, [P.USERS_READ]))).toBe(true);
  });

  it('trang công khai luôn vào được', () => {
    expect(resolveGuard(route('/trang-thai', {}), auth(false))).toBe(true);
  });
});

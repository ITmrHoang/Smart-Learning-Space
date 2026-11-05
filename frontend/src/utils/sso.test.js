import { describe, expect, it } from 'vitest';

import { parseLinkFragment, safeNext, sessionEndedMessage, ssoErrorMessage } from './sso';

describe('SSO', () => {
  it('đọc yêu cầu liên kết trong phần # của URL', () => {
    const info = parseLinkFragment('#token=abc_DEF-123&provider=google&email=l***%40example.com&password=1&expiresIn=900&next=%2Flop-hoc');
    expect(info).toEqual({
      token: 'abc_DEF-123',
      provider: 'google',
      emailHint: 'l***@example.com',
      canUsePassword: true,
      expiresIn: 900,
      next: '/lop-hoc',
    });
  });

  it('thiếu token → null; nhà cung cấp lạ và next ngoài trang bị bỏ', () => {
    expect(parseLinkFragment('')).toBeNull();
    expect(parseLinkFragment('#provider=google')).toBeNull();
    const info = parseLinkFragment('#token=t&provider=facebook&password=0&next=%2F%2Fevil.com');
    expect(info.provider).toBeNull();
    expect(info.canUsePassword).toBe(false);
    expect(info.next).toBeUndefined();
  });

  it('chỉ nhận đường dẫn nội bộ', () => {
    expect(safeNext('/ho-so')).toBe('/ho-so');
    for (const bad of ['//evil.com', '/\\evil.com', 'https://evil.com', 'ho-so', null, undefined]) {
      expect(safeNext(bad)).toBeUndefined();
    }
  });

  it('thông báo lỗi theo mã', () => {
    expect(ssoErrorMessage('SSO_CANCELLED')).toBe('Bạn đã hủy đăng nhập.');
    expect(ssoErrorMessage('MA_LA')).toContain('không thành công');
    expect(ssoErrorMessage('')).toBe('');
    expect(sessionEndedMessage('SESSION_LIMIT')).toContain('thiết bị khác');
    expect(sessionEndedMessage('NO_REFRESH_TOKEN')).toBe('');
  });
});
